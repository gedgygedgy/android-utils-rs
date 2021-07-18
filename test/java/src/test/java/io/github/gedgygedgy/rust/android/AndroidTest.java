package io.github.gedgygedgy.rust.android;

import io.github.gedgygedgy.rust.panic.PanicException;

import org.junit.Test;
import static org.junit.Assert.assertTrue;
import org.junit.runner.RunWith;

import org.robolectric.RobolectricTestRunner;

@RunWith(RobolectricTestRunner.class)
public class AndroidTest {
    public static void loadAndroidUtilsTestLibrary() {
        System.loadLibrary("android_utils_test");
    }

    static {
        loadAndroidUtilsTestLibrary();
    }

    @Test
    public void testPanic() {
        boolean thrown = false;
        try {
            this.testPanicInternal();
        } catch (PanicException ex) {
            assertTrue(ex.getMessage().equals("testPanic() panicked"));
            thrown = true;
        }
        assertTrue(thrown);
    }

    private native void testPanicInternal();
}