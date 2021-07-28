package io.github.gedgygedgy.rust.android.app;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

import io.github.gedgygedgy.rust.ops.FnFunction;
import io.github.gedgygedgy.rust.ops.FnRunnable;

import java.util.HashMap;

/**
 * Base class for {@link Service}s that are implemented in Rust. Extend this
 * class and register its methods with
 * {@code android_utils::service::register_service()}.
 */
public class RustService extends Service {
    private static class OnStartArguments {
        public Intent intent;
        public int flags;
        public int startId;
    }

    private static final HashMap<Class<? extends RustService>, FnFunction<RustService, Void>> onCreateHooks = new HashMap<>();

    private FnFunction<OnStartArguments, Integer> onStartCommandHook;
    private FnFunction<Intent, IBinder> onBindHook;
    private FnFunction<Intent, Boolean> onUnbindHook;
    private FnFunction<Intent, Void> onRebindHook;

    @Override
    public void onCreate() {
        onCreateHooks.get(this.getClass()).apply(this);
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        OnStartArguments args = new OnStartArguments();
        args.intent = intent;
        args.flags = flags;
        args.startId = startId;
        return this.onStartCommandHook.apply(args);
    }

    @Override
    public IBinder onBind(Intent intent) {
        return this.onBindHook.apply(intent);
    }

    @Override
    public boolean onUnbind(Intent intent) {
        return this.onUnbindHook.apply(intent);
    }

    @Override
    public void onRebind(Intent intent) {
        this.onRebindHook.apply(intent);
    }

    @Override
    public void onDestroy() {
        this.onStartCommandHook.close();
        this.onBindHook.close();
        this.onUnbindHook.close();
        this.onRebindHook.close();
    }
}
