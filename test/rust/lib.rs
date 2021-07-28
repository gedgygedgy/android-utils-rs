use android_utils::{
    os::{async_handler_callback, JHandler},
    service::{async_service_connection, register_service, RustService, ServiceConnectionEvent},
};
use futures::StreamExt;
use jni::{
    objects::{GlobalRef, JObject},
    sys::jint,
    JNIEnv, JavaVM,
};
use jni_utils::{
    exceptions::{throw_unwind, try_block},
    ops::fn_once_runnable,
    stream::JSendStream,
};
use std::{
    convert::TryFrom,
    ffi::c_void,
    sync::{Arc, Mutex},
};

#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, _res: *const c_void) -> jint {
    let env = vm.get_env().unwrap();
    android_utils::init(&env).unwrap();
    log::set_max_level(log::LevelFilter::Off);
    jni_utils::init(&env).unwrap();
    jni::JNIVersion::V6.into()
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_AndroidTest_testPanicInternal(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        panic!("testPanic() panicked");
    });
}

fn shadow_looper_and_handler<'a: 'b, 'b>(env: &'b JNIEnv<'a>) -> (JObject<'a>, JHandler<'a, 'b>) {
    let looper = env
        .call_static_method(
            "android/os/Looper",
            "getMainLooper",
            "()Landroid/os/Looper;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let shadow_looper = env
        .call_static_method(
            "org/robolectric/Shadows",
            "shadowOf",
            "(Landroid/os/Looper;)Lorg/robolectric/shadows/ShadowLooper;",
            &[looper.into()],
        )
        .unwrap()
        .l()
        .unwrap();

    let handler = env
        .new_object(
            "android/os/Handler",
            "(Landroid/os/Looper;)V",
            &[looper.into()],
        )
        .unwrap();
    let handler = JHandler::from_env(&env, handler).unwrap();

    (shadow_looper, handler)
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testPost(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let arc = Arc::new(Mutex::new(false));
        let arc2 = arc.clone();
        let runnable = jni_utils::ops::fn_once_runnable(&env, move |_e, _o| {
            let mut guard = arc2.lock().unwrap();
            *guard = true;
        })
        .unwrap();

        handler.post(runnable).unwrap();
        {
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, false);
        }

        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, true);
        }
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawn(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::{channel::oneshot::channel, task::SpawnExt};

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let (sender, receiver) = channel::<()>();

        let arc = Arc::new(Mutex::new(0));
        let arc2 = arc.clone();

        let closure = async move {
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 1;
            }
            receiver.await.unwrap();
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 2;
            }
        };

        let handler_spawn = handler.spawner();

        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        handler_spawn.spawn(closure).unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 0);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        sender.send(()).unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 1);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 2);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawnAsyncSleep(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::task::SpawnExt;
        use std::time::Duration;

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let arc = Arc::new(Mutex::new(0));
        let arc2 = arc.clone();

        let closure = async move {
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 1;
            }
            async_std::task::sleep(Duration::from_millis(500)).await;
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 2;
            }
        };

        let handler_spawn = handler.spawner();

        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        handler_spawn.spawn(closure).unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 0);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );

        // Wait until after the sleep() has completed. This is admittedly a bit
        // racy.
        std::thread::sleep(Duration::from_millis(1000));

        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 1);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 2);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawnNativeThreadWake(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::{channel::oneshot::channel, task::SpawnExt};
        use std::time::Duration;

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let (sender, receiver) = channel::<()>();

        let arc = Arc::new(Mutex::new(0));
        let arc2 = arc.clone();

        let closure = async move {
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 1;
            }
            receiver.await.unwrap();
            {
                let mut guard = arc2.lock().unwrap();
                *guard = 2;
            }
        };

        let handler_spawn = handler.spawner();

        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        handler_spawn.spawn(closure).unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 0);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );

        std::thread::spawn(|| {
            sender.send(()).unwrap();
        })
        .join()
        .unwrap();

        // Wait until after the Runnable has posted. This is admittedly a bit
        // racy.
        std::thread::sleep(Duration::from_millis(500));

        {
            assert_eq!(Arc::strong_count(&arc), 2);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Arc::strong_count(&arc), 1);
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, 2);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawnThread(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::task::SpawnExt;

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);
        let handler_ref = env.new_global_ref(handler).unwrap();

        let runnable = fn_once_runnable(&env, move |env, _obj| {
            let handler = JHandler::from_env(env, handler_ref.as_obj()).unwrap();
            let handler_spawn = handler.spawner();

            handler_spawn.spawn(async {}).unwrap();
        })
        .unwrap();
        let thread = env
            .new_object(
                "java/lang/Thread",
                "(Ljava/lang/Runnable;)V",
                &[runnable.into()],
            )
            .unwrap();
        env.call_method(thread, "start", "()V", &[]).unwrap();
        env.call_method(thread, "join", "()V", &[]).unwrap();

        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawnLocal(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::{channel::oneshot::channel, task::LocalSpawnExt};
        use std::{cell::RefCell, rc::Rc};

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let (sender, receiver) = channel::<()>();

        let rc = Rc::new(RefCell::new(0));
        let rc2 = rc.clone();

        let closure = async move {
            {
                let mut guard = rc2.borrow_mut();
                *guard = 1;
            }
            receiver.await.unwrap();
            {
                let mut guard = rc2.borrow_mut();
                *guard = 2;
            }
        };

        let handler_spawn = handler.spawner();

        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        handler_spawn.spawn_local(closure).unwrap();
        {
            assert_eq!(Rc::strong_count(&rc), 2);
            let guard = rc.borrow();
            assert_eq!(*guard, 0);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Rc::strong_count(&rc), 2);
            let guard = rc.borrow();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
        sender.send(()).unwrap();
        {
            assert_eq!(Rc::strong_count(&rc), 2);
            let guard = rc.borrow();
            assert_eq!(*guard, 1);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            false
        );
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            assert_eq!(Rc::strong_count(&rc), 1);
            let guard = rc.borrow();
            assert_eq!(*guard, 2);
        }
        assert_eq!(
            env.call_method(shadow_looper, "isIdle", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            true
        );
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testSpawnLocalThread(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::task::LocalSpawnExt;

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);
        let handler_ref = env.new_global_ref(handler).unwrap();

        let runnable = fn_once_runnable(&env, move |env, _obj| {
            let handler = JHandler::from_env(env, handler_ref.as_obj()).unwrap();
            let handler_spawn = handler.spawner();

            handler_spawn.spawn_local(async {}).unwrap();
        })
        .unwrap();
        let thread = env
            .new_object(
                "java/lang/Thread",
                "(Ljava/lang/Runnable;)V",
                &[runnable.into()],
            )
            .unwrap();
        env.call_method(thread, "start", "()V", &[]).unwrap();
        env.call_method(thread, "join", "()V", &[]).unwrap();

        let result = try_block(&env, || {
            env.call_method(shadow_looper, "runOneTask", "()V", &[])?;
            Ok(false)
        })
        .catch(
            "io/github/gedgygedgy/rust/thread/LocalThreadException",
            |_ex| Ok(true),
        )
        .result();
        assert_eq!(result.unwrap(), true);
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testRustHandlerCallback(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::task::SpawnExt;

        let handler_data = Arc::new(Mutex::new(None));

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let (callback, stream) = async_handler_callback(&env).unwrap();
        let mut stream = JSendStream::try_from(stream).unwrap();

        let handler_data_clone = handler_data.clone();
        let task = async move {
            let msg = stream.next().await.unwrap().unwrap();
            let mut guard = handler_data_clone.lock().unwrap();
            *guard = Some(msg);
        };

        handler.spawner().spawn(task).unwrap();
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            let guard = handler_data.lock().unwrap();
            assert!(guard.is_none());
        }

        let message = env
            .call_static_method(
                "android/os/Message",
                "obtain",
                "()Landroid/os/Message;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();
        let result = env
            .call_method(
                callback,
                "handleMessage",
                "(Landroid/os/Message;)Z",
                &[message.into()],
            )
            .unwrap()
            .z()
            .unwrap();
        assert_eq!(result, false);

        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            let guard = handler_data.lock().unwrap();
            assert!(env
                .is_same_object(message, guard.as_ref().unwrap().as_obj())
                .unwrap());
        }
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_ServiceTest_testRustServiceConnection(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use futures::task::SpawnExt;

        let (conn, mut stream) = async_service_connection(&env).unwrap();

        let (shadow_looper, handler) = shadow_looper_and_handler(&env);

        let pkg = env.new_string("io.github.gedgygedgy.rust.android").unwrap();
        let cls = env
            .new_string("io.github.gedgygedgy.rust.android.ServiceTest$TestService")
            .unwrap();
        let component_name = env
            .new_object(
                "android/content/ComponentName",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[pkg.into(), cls.into()],
            )
            .unwrap();
        let component_name_ref = env.new_global_ref(component_name).unwrap();

        let messenger = env
            .new_object(
                "android/os/Messenger",
                "(Landroid/os/Handler;)V",
                &[handler.clone().into()],
            )
            .unwrap();
        let service = env
            .call_method(messenger, "getBinder", "()Landroid/os/IBinder;", &[])
            .unwrap()
            .l()
            .unwrap();
        let service_ref = env.new_global_ref(service).unwrap();

        let vm = env.get_java_vm().unwrap();
        let finished = Arc::new(Mutex::new(false));
        let finished_clone = finished.clone();
        let task = async move {
            let item = stream.next().await.unwrap().unwrap();
            if let ServiceConnectionEvent::BindingDied { component_name } = item {
                let env = vm.get_env().unwrap();
                assert!(env
                    .is_same_object(component_name.as_obj(), component_name_ref.as_obj())
                    .unwrap());
            } else {
                panic!("Expected BindingDied");
            }

            let item = stream.next().await.unwrap().unwrap();
            if let ServiceConnectionEvent::NullBinding { component_name } = item {
                let env = vm.get_env().unwrap();
                assert!(env
                    .is_same_object(component_name.as_obj(), component_name_ref.as_obj())
                    .unwrap());
            } else {
                panic!("Expected NullBinding");
            }

            let item = stream.next().await.unwrap().unwrap();
            if let ServiceConnectionEvent::ServiceConnected {
                component_name,
                service,
            } = item
            {
                let env = vm.get_env().unwrap();
                assert!(env
                    .is_same_object(component_name.as_obj(), component_name_ref.as_obj())
                    .unwrap());
                assert!(env
                    .is_same_object(service.as_obj(), service_ref.as_obj())
                    .unwrap());
            } else {
                panic!("Expected ServiceConnected");
            }

            let item = stream.next().await.unwrap().unwrap();
            if let ServiceConnectionEvent::ServiceDisconnected { component_name } = item {
                let env = vm.get_env().unwrap();
                assert!(env
                    .is_same_object(component_name.as_obj(), component_name_ref.as_obj())
                    .unwrap());
            } else {
                panic!("Expected ServiceDisconnected");
            }

            let mut guard = finished_clone.lock().unwrap();
            *guard = true;
        };

        handler.spawner().spawn(task).unwrap();

        env.call_method(
            conn,
            "onBindingDied",
            "(Landroid/content/ComponentName;)V",
            &[component_name.into()],
        )
        .unwrap();
        env.call_method(
            conn,
            "onNullBinding",
            "(Landroid/content/ComponentName;)V",
            &[component_name.into()],
        )
        .unwrap();
        env.call_method(
            conn,
            "onServiceConnected",
            "(Landroid/content/ComponentName;Landroid/os/IBinder;)V",
            &[component_name.into(), service.into()],
        )
        .unwrap();
        env.call_method(
            conn,
            "onServiceDisconnected",
            "(Landroid/content/ComponentName;)V",
            &[component_name.into()],
        )
        .unwrap();

        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            let guard = finished.lock().unwrap();
            assert!(*guard);
        }
    });
}

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_ServiceTest_testRustService(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        struct TestServiceData {
            created: bool,
            binder: GlobalRef,
            intent: Option<GlobalRef>,
            rebound: bool,
            start_flags: Option<jint>,
            start_id: Option<jint>,
        }

        struct TestService(Arc<Mutex<TestServiceData>>);

        impl Drop for TestService {
            fn drop(&mut self) {
                let mut guard = self.0.lock().unwrap();
                guard.created = false;
            }
        }

        impl RustService for TestService {
            fn on_bind<'a: 'b, 'b>(&self, env: &'b JNIEnv<'a>, intent: JObject<'a>) -> JObject<'a> {
                let mut guard = self.0.lock().unwrap();
                guard.intent = Some(env.new_global_ref(intent).unwrap());
                guard.binder.as_obj().into_inner().into()
            }

            fn on_unbind<'a: 'b, 'b>(&self, _env: &'b JNIEnv<'a>, _intent: JObject<'a>) -> bool {
                let mut guard = self.0.lock().unwrap();
                guard.intent = None;
                true
            }

            fn on_rebind<'a: 'b, 'b>(&self, env: &'b JNIEnv<'a>, intent: JObject<'a>) {
                let mut guard = self.0.lock().unwrap();
                guard.intent = Some(env.new_global_ref(intent).unwrap());
                guard.rebound = true;
            }

            fn on_start_command<'a: 'b, 'b>(
                &self,
                _env: &'b JNIEnv<'a>,
                _intent: JObject<'a>,
                flags: jint,
                start_id: jint,
            ) -> jint {
                let mut guard = self.0.lock().unwrap();
                guard.start_id = Some(start_id);
                guard.start_flags = Some(flags);
                android_utils::service::START_STICKY
            }
        }

        let (_shadow_looper, handler) = shadow_looper_and_handler(&env);
        let messenger = env
            .new_object(
                "android/os/Messenger",
                "(Landroid/os/Handler;)V",
                &[handler.into()],
            )
            .unwrap();
        let binder = env
            .call_method(messenger, "getBinder", "()Landroid/os/IBinder;", &[])
            .unwrap()
            .l()
            .unwrap();
        let binder_ref = env.new_global_ref(binder).unwrap();

        let data = Arc::new(Mutex::new(TestServiceData {
            created: false,
            binder: binder_ref.clone(),
            intent: None,
            rebound: false,
            start_flags: None,
            start_id: None,
        }));
        let data_clone = data.clone();

        let factory = move |_env: &JNIEnv, _obj: JObject| {
            {
                let mut guard = data_clone.lock().unwrap();
                guard.created = true;
            }
            TestService(data_clone.clone())
        };

        let class = env
            .find_class("io/github/gedgygedgy/rust/android/ServiceTest$TestRustService")
            .unwrap();
        register_service(&env, class, factory).unwrap();

        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, false);
            assert!(guard.intent.is_none());
        }

        let context = env
            .call_static_method(
                "androidx/test/core/app/ApplicationProvider",
                "getApplicationContext",
                "()Landroid/content/Context;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();
        let class = env
            .find_class("io/github/gedgygedgy/rust/android/ServiceTest$TestRustService")
            .unwrap();
        let intent = env
            .new_object(
                "android/content/Intent",
                "(Landroid/content/Context;Ljava/lang/Class;)V",
                &[context.into(), class.into()],
            )
            .unwrap();

        let service = env.new_object(class, "()V", &[]).unwrap();

        let service_controller = env.call_static_method(
            "org/robolectric/android/controller/ServiceController",
            "of",
            "(Landroid/app/Service;Landroid/content/Intent;)Lorg/robolectric/android/controller/ServiceController;",
            &[service.into(), intent.into()],
        )
           .unwrap().l().unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, false);
            assert!(guard.intent.is_none());
        }

        env.call_method(
            service_controller,
            "create",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, true);
            assert!(guard.intent.is_none());
            assert!(guard.start_id.is_none());
        }

        env.call_method(
            service_controller,
            "bind",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, true);
            assert!(env
                .is_same_object(guard.intent.as_ref().unwrap(), intent)
                .unwrap());
            assert_eq!(guard.rebound, false);
            assert!(guard.start_id.is_none());
        }

        env.call_method(
            service_controller,
            "unbind",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, true);
            assert!(guard.intent.is_none());
            assert_eq!(guard.rebound, false);
            assert!(guard.start_id.is_none());
        }

        env.call_method(
            service_controller,
            "rebind",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, true);
            assert!(env
                .is_same_object(guard.intent.as_ref().unwrap(), intent)
                .unwrap());
            assert_eq!(guard.rebound, true);
            assert!(guard.start_id.is_none());
        }

        env.call_method(
            service_controller,
            "unbind",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, true);
            assert!(guard.intent.is_none());
            assert_eq!(guard.rebound, true);
            assert!(guard.start_id.is_none());
        }

        env.call_method(
            service_controller,
            "startCommand",
            "(II)Lorg/robolectric/android/controller/ServiceController;",
            &[android_utils::service::START_FLAG_RETRY.into(), 42.into()],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.start_id.unwrap(), 42);
            assert_eq!(
                guard.start_flags.unwrap(),
                android_utils::service::START_FLAG_RETRY
            );
        }

        env.call_method(
            service_controller,
            "destroy",
            "()Lorg/robolectric/android/controller/ServiceController;",
            &[],
        )
        .unwrap();
        {
            let guard = data.lock().unwrap();
            assert_eq!(guard.created, false);
        }
    });
}
