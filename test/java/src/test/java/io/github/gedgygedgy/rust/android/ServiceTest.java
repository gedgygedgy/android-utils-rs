package io.github.gedgygedgy.rust.android;

import org.junit.Test;
import org.junit.runner.RunWith;

import org.robolectric.RobolectricTestRunner;

import io.github.gedgygedgy.rust.android.app.RustService;

@RunWith(RobolectricTestRunner.class)
public class ServiceTest {
    private static class TestRustService extends RustService {}

    static {
        AndroidTest.loadAndroidUtilsTestLibrary();
    }

    @Test
    public native void testRustServiceConnection();

    @Test
    public native void testRustService();
}
