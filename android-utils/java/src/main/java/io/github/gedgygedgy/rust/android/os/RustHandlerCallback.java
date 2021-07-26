package io.github.gedgygedgy.rust.android.os;

import android.os.Handler;
import android.os.Message;

import io.github.gedgygedgy.rust.stream.QueueStream;
import io.github.gedgygedgy.rust.stream.Stream;

final class RustHandlerCallback implements Handler.Callback {
    private final QueueStream<Message> stream = new QueueStream<>();

    @Override
    public boolean handleMessage(Message message) {
        this.stream.add(message);
        return false;
    }

    public Stream<Message> getMessageStream() {
        return this.stream;
    }
}
