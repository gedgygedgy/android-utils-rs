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
    sync::Arc,
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

struct HandlerWaker {
    vm: JavaVM,
    handler: GlobalRef,
    runnable: GlobalRef,
}

impl Wake for HandlerWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let env = self.vm.get_env().unwrap();
        let handler = JHandler::from_env(&env, self.handler.as_obj()).unwrap();
        handler.post(self.runnable.as_obj()).unwrap();
    }
}

/// Object that implements [`Spawn`] and [`LocalSpawn`] for [`JHandler`].
/// Obtained by calling [`JHandler::spawner`].
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
