use jni::{objects::JObject, sys::jint, JNIEnv, JavaVM};
use jni_utils::exceptions::throw_unwind;
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

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_HandlerTest_testPost(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        use android_utils::os::JHandler;
        use std::sync::{Arc, Mutex};

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

        let handler = env
            .new_object(
                "android/os/Handler",
                "(Landroid/os/Looper;)V",
                &[looper.into()],
            )
            .unwrap();
        let handler = JHandler::from_env(&env, handler).unwrap();

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
        env.call_method(shadow_looper, "runOneTask", "()V", &[])
            .unwrap();
        {
            let guard = arc.lock().unwrap();
            assert_eq!(*guard, true);
        }
    });
}
