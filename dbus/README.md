# printnanny-dbus

### XML Introspection

```
busctl --system --xml-interface introspect \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager > org.freedesktop.systemd1.Manager.xml
```

```
busctl --system --xml-interface introspect \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1/unit/avahi_2ddaemon_2eservice org.freedesktop.systemd1.Service > org.freedesktop.systemd1.Service.xml
```

```
busctl --system --xml-interface introspect \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1/unit/basic_2etarget org.freedesktop.systemd1.Target > org.freedesktop.systemd1.Target.xml
```

```
busctl --system --xml-interface introspect \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1/job/1292 org.freedesktop.systemd1.Job > org.freedesktop.systemd1.Job.xml
```