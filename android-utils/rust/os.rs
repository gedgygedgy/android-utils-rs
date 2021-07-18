use jni::{
    errors::Result,
    objects::{JMethodID, JObject},
    signature::{JavaType, Primitive},
    JNIEnv,
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
}
