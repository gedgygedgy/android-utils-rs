use futures::{Stream, StreamExt};
use jni::{
    descriptors::Desc,
    errors::Result,
    objects::{GlobalRef, JClass, JObject},
    sys::jint,
    JNIEnv,
};
use jni_utils::stream::{JSendStream, JStream};
use std::{convert::TryFrom, sync::Arc};

/// Represents events that have been captured by an
/// `android.content.ServiceConnection`.
pub enum ServiceConnectionEvent {
    /// Created by `ServiceConnection.onBindingDied()`.
    BindingDied { component_name: GlobalRef },
    /// Created by `ServiceConnection.onNullBinding()`.
    NullBinding { component_name: GlobalRef },
    /// Created by `ServiceConnection.onServiceConnected()`.
    ServiceConnected {
        component_name: GlobalRef,
        service: GlobalRef,
    },
    /// Created by `ServiceConnection.onServiceDisconnected()`.
    ServiceDisconnected { component_name: GlobalRef },
}

/// Creates an `android.content.ServiceConnection` and an accompanying stream
/// of [`ServiceConnectionEvent`]s captured by it.
pub fn async_service_connection<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
) -> Result<(
    JObject<'a>,
    impl Stream<Item = Result<ServiceConnectionEvent>> + Send,
)> {
    let vm = env.get_java_vm()?;
    let conn = env.new_object(
        "io/github/gedgygedgy/rust/android/content/RustServiceConnection",
        "()V",
        &[],
    )?;
    let stream = env
        .call_method(
            conn,
            "getEventStream",
            "()Lio/github/gedgygedgy/rust/stream/Stream;",
            &[],
        )?
        .l()?;
    let stream = JSendStream::try_from(JStream::from_env(env, stream)?)?;

    let mapped_stream = stream.map(move |item| -> Result<_> {
        let item = item?;
        let item = item.as_obj();
        let env = vm.get_env()?;

        if env.is_instance_of(
            item,
            "io/github/gedgygedgy/rust/android/content/RustServiceConnection$BindingDiedEvent",
        )? {
            let name = env.new_global_ref(
                env.get_field(item, "name", "Landroid/content/ComponentName;")?
                    .l()?,
            )?;
            Ok(ServiceConnectionEvent::BindingDied {
                component_name: name,
            })
        } else if env.is_instance_of(
            item,
            "io/github/gedgygedgy/rust/android/content/RustServiceConnection$NullBindingEvent",
        )? {
            let name = env.new_global_ref(
                env.get_field(item, "name", "Landroid/content/ComponentName;")?
                    .l()?,
            )?;
            Ok(ServiceConnectionEvent::NullBinding {
                component_name: name,
            })
        } else if env.is_instance_of(
            item,
            "io/github/gedgygedgy/rust/android/content/RustServiceConnection$ServiceConnectedEvent",
        )? {
            let name = env.new_global_ref(
                env.get_field(item, "name", "Landroid/content/ComponentName;")?
                    .l()?,
            )?;
            let service = env.new_global_ref(
                env.get_field(item, "service", "Landroid/os/IBinder;")?
                    .l()?,
            )?;
            Ok(ServiceConnectionEvent::ServiceConnected {
                component_name: name,
                service,
            })
        } else if env.is_instance_of(
            item,
            "io/github/gedgygedgy/rust/android/content/RustServiceConnection$ServiceDisconnectedEvent",
        )? {
            let name = env.new_global_ref(
                env.get_field(item, "name", "Landroid/content/ComponentName;")?
                    .l()?,
            )?;
            Ok(ServiceConnectionEvent::ServiceDisconnected {
                component_name: name,
            })
        } else {
            panic!("Unknown Event class");
        }
    });

    Ok((conn, mapped_stream))
}

/// `android.app.Service.START_FLAG_REDELIVERY`.
pub const START_FLAG_REDELIVERY: jint = 1;

/// `android.app.Service.START_FLAG_RETRY`.
pub const START_FLAG_RETRY: jint = 2;

/// `android.app.Service.START_STICKY_COMPATIBILITY`.
pub const START_STICKY_COMPATIBIILITY: jint = 0;

/// `android.app.Service.START_STICKY`.
pub const START_STICKY: jint = 1;

/// `android.app.Service.START_NOT_STICKY`.
pub const START_NOT_STICKY: jint = 2;

/// `android.app.Service.START_REDELIVER_INTENT`.
pub const START_REDELIVER_INTENT: jint = 3;

/// Trait for Rust implementations of `android.app.Service`. Register your
/// Rust service using [`register_service`].
#[allow(unused_variables)]
pub trait RustService: Send + Sync {
    /// Called by `Service.onStartCommand()`.
    fn on_start_command<'a: 'b, 'b>(
        &self,
        env: &'b JNIEnv<'a>,
        intent: JObject<'a>,
        flags: jint,
        start_id: jint,
    ) -> jint {
        START_STICKY
    }

    /// Called by `Service.onBind()`.
    fn on_bind<'a: 'b, 'b>(&self, env: &'b JNIEnv<'a>, intent: JObject<'a>) -> JObject<'a>;

    /// Called by `Service.onUnbind()`.
    fn on_unbind<'a: 'b, 'b>(&self, env: &'b JNIEnv<'a>, intent: JObject<'a>) -> bool {
        false
    }

    /// Called by `Service.onRebind()`.
    fn on_rebind<'a: 'b, 'b>(&self, env: &'b JNIEnv<'a>, intent: JObject<'a>) {}
}

/// Register a service as an
/// `io.github.gedgygedgy.rust.android.app.RustService`. The `factory` closure
/// is called when `Service.onCreate()` is called, and the object created by it
/// is dropped when `Service.onDestroy()` is called.
pub fn register_service<'a: 'b, 'b, T: RustService + 'static>(
    env: &'b JNIEnv<'a>,
    class: impl Desc<'a, JClass<'a>>,
    factory: impl for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) -> T + Send + Sync + 'static,
) -> Result<()> {
    let class = env.auto_local(class.lookup(env)?);

    let on_create_hook =
        env.auto_local(jni_utils::ops::fn_function(env, move |env, _obj, arg| {
            let service = Arc::new(factory(env, arg));

            let service_clone = service.clone();
            let on_start_command_hook = env.auto_local(
                jni_utils::ops::fn_function(env, move |env, _obj, arg| {
                    let intent = env.auto_local(
                        env.get_field(arg, "intent", "Landroid/content/Intent;")
                            .unwrap()
                            .l()
                            .unwrap(),
                    );
                    let flags = env.get_field(arg, "flags", "I").unwrap().i().unwrap();
                    let start_id = env.get_field(arg, "startId", "I").unwrap().i().unwrap();

                    let result =
                        service_clone.on_start_command(env, (&intent).into(), flags, start_id);
                    let result_obj = env
                        .new_object("java/lang/Integer", "(I)V", &[result.into()])
                        .unwrap();
                    result_obj
                })
                .unwrap(),
            );
            env.set_field(
                arg,
                "onStartCommandHook",
                "Lio/github/gedgygedgy/rust/ops/FnFunction;",
                (&on_start_command_hook).into(),
            )
            .unwrap();

            let service_clone = service.clone();
            let on_bind_hook = env.auto_local(
                jni_utils::ops::fn_function(env, move |env, _obj, arg| {
                    service_clone.on_bind(env, arg)
                })
                .unwrap(),
            );
            env.set_field(
                arg,
                "onBindHook",
                "Lio/github/gedgygedgy/rust/ops/FnFunction;",
                (&on_bind_hook).into(),
            )
            .unwrap();

            let service_clone = service.clone();
            let on_unbind_hook = env.auto_local(
                jni_utils::ops::fn_function(env, move |env, _obj, arg| {
                    let result = service_clone.on_unbind(env, arg);
                    let result_obj = env
                        .new_object("java/lang/Boolean", "(Z)V", &[result.into()])
                        .unwrap();
                    result_obj
                })
                .unwrap(),
            );
            env.set_field(
                arg,
                "onUnbindHook",
                "Lio/github/gedgygedgy/rust/ops/FnFunction;",
                (&on_unbind_hook).into(),
            )
            .unwrap();

            let on_rebind_hook = env.auto_local(
                jni_utils::ops::fn_function(env, move |env, _obj, arg| {
                    service.on_rebind(env, arg);
                    JObject::null()
                })
                .unwrap(),
            );
            env.set_field(
                arg,
                "onRebindHook",
                "Lio/github/gedgygedgy/rust/ops/FnFunction;",
                (&on_rebind_hook).into(),
            )
            .unwrap();

            JObject::null()
        })?);

    let on_create_hooks = env.auto_local(
        env.get_static_field(
            "io/github/gedgygedgy/rust/android/app/RustService",
            "onCreateHooks",
            "Ljava/util/HashMap;",
        )?
        .l()?,
    );
    env.call_method(
        &on_create_hooks,
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        &[(&class).into(), (&on_create_hook).into()],
    )?;

    Ok(())
}

/// Unregister a service as an
/// `io.github.gedgygedgy.rust.android.app.RustService`.
pub fn unregister_service<'a: 'b, 'b, T: RustService + 'static>(
    env: &'b JNIEnv<'a>,
    class: impl Desc<'a, JClass<'a>>,
) -> Result<()> {
    let class = env.auto_local(class.lookup(env)?);

    let on_create_hooks = env.auto_local(
        env.get_static_field(
            "io/github/gedgygedgy/rust/android/app/RustService",
            "onCreateHooks",
            "Ljava/util/HashMap;",
        )?
        .l()?,
    );
    let on_create_hook = env
        .call_method(
            &on_create_hooks,
            "remove",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            &[(&class).into()],
        )?
        .l()?;
    env.call_method(on_create_hook, "close", "()V", &[])?;

    Ok(())
}
