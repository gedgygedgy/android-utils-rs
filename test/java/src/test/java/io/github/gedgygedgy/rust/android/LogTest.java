package io.github.gedgygedgy.rust.android;

import org.junit.Test;
import org.junit.runner.RunWith;

import org.robolectric.RobolectricTestRunner;

@RunWith(RobolectricTestRunner.class)
public class LogTest {
    static {
        AndroidTest.loadAndroidUtilsTestLibrary();
    }

    @Test
    public native void testIsLoggable();

    @Test
    public native void testLog();
}
