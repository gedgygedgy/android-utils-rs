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
}
