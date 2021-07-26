package io.github.gedgygedgy.rust.android.content;

import android.content.ComponentName;
import android.content.ServiceConnection;

import android.os.IBinder;

import io.github.gedgygedgy.rust.stream.QueueStream;
import io.github.gedgygedgy.rust.stream.Stream;

final class RustServiceConnection implements ServiceConnection {
    public static class Event {}

    public static class BindingDiedEvent extends Event {
        public final ComponentName name;

        public BindingDiedEvent(ComponentName name) {
            this.name = name;
        }
    }

    public static class NullBindingEvent extends Event {
        public final ComponentName name;

        public NullBindingEvent(ComponentName name) {
            this.name = name;
        }
    }

    public static class ServiceConnectedEvent extends Event {
        public final ComponentName name;
        public final IBinder service;

        public ServiceConnectedEvent(ComponentName name, IBinder service) {
            this.name = name;
            this.service = service;
        }
    }

    public static class ServiceDisconnectedEvent extends Event {
        public final ComponentName name;

        public ServiceDisconnectedEvent(ComponentName name) {
            this.name = name;
        }
    }

    private final QueueStream<Event> stream = new QueueStream<>();

    public Stream<Event> getEventStream() {
        return this.stream;
    }

    @Override
    public void onBindingDied(ComponentName name) {
        this.stream.add(new BindingDiedEvent(name));
    }

    @Override
    public void onNullBinding(ComponentName name) {
        this.stream.add(new NullBindingEvent(name));
    }

    @Override
    public void onServiceConnected(ComponentName name, IBinder service) {
        this.stream.add(new ServiceConnectedEvent(name, service));
    }

    @Override
    public void onServiceDisconnected(ComponentName name) {
        this.stream.add(new ServiceDisconnectedEvent(name));
    }
}
