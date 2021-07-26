package io.github.gedgygedgy.rust.android;

import org.junit.Test;
import org.junit.runner.RunWith;

import org.robolectric.RobolectricTestRunner;

@RunWith(RobolectricTestRunner.class)
public class HandlerTest {
    static {
        AndroidTest.loadAndroidUtilsTestLibrary();
    }

    @Test
    public native void testPost();

    @Test
    public native void testSpawn();

    @Test
    public native void testSpawnNativeThreadWake();

    @Test
    public native void testSpawnAsyncSleep();

    @Test
    public native void testSpawnThread();

    @Test
    public native void testSpawnLocal();

    @Test
    public native void testSpawnLocalThread();

    @Test
    public native void testRustHandlerCallback();
}
