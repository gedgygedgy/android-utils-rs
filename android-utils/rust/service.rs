use futures::{Stream, StreamExt};
use jni::{
    errors::Result,
    objects::{GlobalRef, JObject},
    JNIEnv,
};
use jni_utils::stream::{JSendStream, JStream};
use std::convert::TryFrom;

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
