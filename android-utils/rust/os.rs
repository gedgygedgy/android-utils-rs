use futures::task::{FutureObj, LocalFutureObj, LocalSpawn, Spawn, SpawnError};
use jni::{
    errors::Result,
    objects::{GlobalRef, JMethodID, JObject},
    signature::{JavaType, Primitive},
    JNIEnv, JavaVM,
};
use once_cell::sync::OnceCell;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake, Waker},
};

/// Wrapper for [`JObject`]s that contain `android.os.Handler`. Provides method
/// to post `java.lang.Runnable`s to the `Handler`.
///
/// Looks up the class and method IDs on creation rather than for every method
/// call.
pub struct JHandler<'a: 'b, 'b> {
    internal: JObject<'a>,
    post: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JHandler<'a, 'b> {
    /// Create a [`JHandler`] from the environment and an object. This looks up
    /// the necessary class and method IDs to call all of the methods on it so
    /// that extra work doesn't need to be done on every method call.
    ///
    /// # Arguments
    ///
    /// * `env` - Java environment to use.
    /// * `obj` - Object to wrap.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("android/os/Handler")?);

        let post = env.get_method_id(&class, "post", "(Ljava/lang/Runnable;)Z")?;
        Ok(Self {
            internal: obj,
            post,
            env,
        })
    }

    /// Post a `java.lang.Runnable` to the `Handler`.
    ///
    /// # Arguments
    ///
    /// * `obj` - `Runnable` to post.
    pub fn post(&self, obj: JObject<'a>) -> Result<bool> {
        self.env
            .call_method_unchecked(
                self.internal,
                self.post,
                JavaType::Primitive(Primitive::Boolean),
                &[obj.into()],
            )?
            .z()
    }

    /// Creates an object that can be used to spawn async functions. The
    /// returned object implements [`Spawn`] and [`LocalSpawn`].
    pub fn spawner(self) -> JHandlerSpawn<'a, 'b> {
        JHandlerSpawn(self)
    }
}

impl<'a: 'b, 'b> From<JHandler<'a, 'b>> for JObject<'a> {
    fn from(handler: JHandler<'a, 'b>) -> Self {
        handler.internal
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JHandler<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

static FALLBACK_SENDER: OnceCell<Mutex<Option<Sender<Arc<HandlerWaker>>>>> = OnceCell::new();

fn init_fallback_thread<'a: 'b, 'b>(env: &'b JNIEnv<'a>) {
    FALLBACK_SENDER.get_or_init(|| {
        let (sender, receiver) = channel::<Arc<HandlerWaker>>();

        let thread_class = env.auto_local(env.find_class("java/lang/Thread").unwrap());
        let thread_ctor = env
            .get_method_id(
                &thread_class,
                "<init>",
                "(Ljava/lang/Runnable;Ljava/lang/String;)V",
            )
            .unwrap();

        let shutdown_runnable = env.auto_local(
            jni_utils::ops::fn_once_runnable(env, |_env, _obj| {
                let mut guard = FALLBACK_SENDER.get().unwrap().lock().unwrap();
                guard.take();
            })
            .unwrap(),
        );
        let shutdown_name = env.auto_local(env.new_string("wake-shutdown").unwrap());
        let shutdown_thread = env.auto_local(
            env.new_object_unchecked(
                &thread_class,
                thread_ctor,
                &[(&shutdown_runnable).into(), (&shutdown_name).into()],
            )
            .unwrap(),
        );
        let runtime = env.auto_local(
            env.call_static_method(
                "java/lang/Runtime",
                "getRuntime",
                "()Ljava/lang/Runtime;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap(),
        );
        env.call_method(
            &runtime,
            "addShutdownHook",
            "(Ljava/lang/Thread;)V",
            &[(&shutdown_thread).into()],
        )
        .unwrap();

        let fallback_runnable = env.auto_local(
            jni_utils::ops::fn_once_runnable(env, move |_env, _obj| {
                for waker in receiver {
                    waker.wake_direct().unwrap();
                }
            })
            .unwrap(),
        );
        let fallback_name = env.auto_local(env.new_string("wake-fallback").unwrap());
        let fallback_thread = env.auto_local(
            env.new_object_unchecked(
                &thread_class,
                thread_ctor,
                &[(&fallback_runnable).into(), (&fallback_name).into()],
            )
            .unwrap(),
        );
        env.call_method(&fallback_thread, "setDaemon", "(Z)V", &[true.into()])
            .unwrap();
        env.call_method(&fallback_thread, "start", "()V", &[])
            .unwrap();

        Mutex::new(Some(sender))
    });
}

struct HandlerWaker {
    vm: JavaVM,
    handler: GlobalRef,
    runnable: GlobalRef,
}

impl HandlerWaker {
    fn wake_direct(&self) -> Result<()> {
        let env = self.vm.get_env()?;
        let handler = JHandler::from_env(&env, self.handler.as_obj())?;
        handler.post(self.runnable.as_obj())?;
        Ok(())
    }

    fn wake_fallback(self: Arc<Self>) {
        let guard = FALLBACK_SENDER.get().unwrap().lock().unwrap();
        guard.as_ref().unwrap().send(self).unwrap();
    }
}

impl Wake for HandlerWaker {
    fn wake(self: Arc<Self>) {
        if self.wake_direct().is_err() {
            self.wake_fallback();
        }
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if self.wake_direct().is_err() {
            self.clone().wake_fallback();
        }
    }
}

/// Object that implements [`Spawn`] and [`LocalSpawn`] for [`JHandler`].
/// Obtained by calling [`JHandler::spawner`].
///
/// The [`Waker`] produced by this object can be woken from a Java thread or
/// a native, non-Java-attached thread. If it's woken from a Java thread, the
/// thread will directly post the task to the `Handler`. If it's woken from a
/// native, non-Java-attached thread, the [`Waker`] will send itself to a
/// fallback thread that is Java-attached and whose sole purpose is to post
/// async tasks to the `Handler` on behalf of native threads. This fallback
/// thread will be started upon spawning an async task from a [`JHandlerSpawn`]
/// for the first time, and will be shut down when the JVM shuts down.
pub struct JHandlerSpawn<'a: 'b, 'b>(JHandler<'a, 'b>);

impl<'a: 'b, 'b> JHandlerSpawn<'a, 'b> {
    fn wrap_future(
        &self,
        mut fut: impl Future<Output = ()> + Unpin,
    ) -> impl for<'c, 'd> FnMut(&'d JNIEnv<'c>, JObject<'c>) {
        let waker_cell = OnceCell::new();
        let handler = self.0.env.new_global_ref(self.0.internal).unwrap();
        move |env, obj| {
            let handler = handler.clone();
            let waker = waker_cell.get_or_init(move || {
                let runnable = env.new_global_ref(obj).unwrap();
                let arc = Arc::new(HandlerWaker {
                    vm: env.get_java_vm().unwrap(),
                    handler,
                    runnable,
                });
                Waker::from(arc)
            });
            let mut context = Context::from_waker(waker);
            let pin = Pin::new(&mut fut);
            match pin.poll(&mut context) {
                Poll::Ready(()) => {
                    env.call_method(obj, "close", "()V", &[]).unwrap();
                }
                Poll::Pending => {}
            }
        }
    }

    fn post_spawn(&self, runnable: JObject<'a>) -> std::result::Result<(), SpawnError> {
        init_fallback_thread(self.0.env);
        if self.0.post(runnable).unwrap() {
            Ok(())
        } else {
            Err(SpawnError::shutdown())
        }
    }
}

impl<'a: 'b, 'b> From<JHandlerSpawn<'a, 'b>> for JHandler<'a, 'b> {
    fn from(spawn: JHandlerSpawn<'a, 'b>) -> Self {
        spawn.0
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JHandlerSpawn<'a, 'b> {
    type Target = JHandler<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a: 'b, 'b> Spawn for JHandlerSpawn<'a, 'b> {
    fn spawn_obj(&self, fut: FutureObj<'static, ()>) -> std::result::Result<(), SpawnError> {
        let runnable = jni_utils::ops::fn_mut_runnable(self.0.env, self.wrap_future(fut)).unwrap();
        self.post_spawn(runnable)
    }
}

impl<'a: 'b, 'b> LocalSpawn for JHandlerSpawn<'a, 'b> {
    fn spawn_local_obj(
        &self,
        fut: LocalFutureObj<'static, ()>,
    ) -> std::result::Result<(), SpawnError> {
        let runnable =
            jni_utils::ops::fn_mut_runnable_local(self.0.env, self.wrap_future(fut)).unwrap();
        self.post_spawn(runnable)
    }
}
