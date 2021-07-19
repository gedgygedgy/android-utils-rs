use android_utils::os::JHandler;
use jni::{objects::JObject, sys::jint, JNIEnv, JavaVM};
use jni_utils::{
    exceptions::{throw_unwind, try_block},
    ops::fn_once_runnable,
};
use std::ffi::c_void;

#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, _res: *const c_void) -> jint {
    let env = vm.get_env().unwrap();
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
        use std::sync::{Arc, Mutex};

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
        use std::sync::{Arc, Mutex};

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
