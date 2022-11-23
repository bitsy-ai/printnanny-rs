use zbus::dbus_proxy;

/// Proxy object for `org.freedesktop.systemd1.Job`.
#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Job",
    gen_async = true,
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait Job {
    /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#Cancel()) Call interface method `Cancel`.
    #[dbus_proxy(name = "Cancel")]
    fn cancel(&self) -> crate::zbus::Result<()>;

    /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#GetAfter()) Call interface method `GetAfter`.
    #[dbus_proxy(name = "GetAfter")]
    fn get_after(
        &self,
    ) -> crate::zbus::Result<
        Vec<(
            u32,
            String,
            String,
            String,
            zbus::zvariant::OwnedObjectPath,
            zbus::zvariant::OwnedObjectPath,
        )>,
    >;

    /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#GetBefore()) Call interface method `GetBefore`.
    #[dbus_proxy(name = "GetBefore")]
    fn get_before(
        &self,
    ) -> crate::zbus::Result<
        Vec<(
            u32,
            String,
            String,
            String,
            zbus::zvariant::OwnedObjectPath,
            zbus::zvariant::OwnedObjectPath,
        )>,
    >;

    /// Get property `Id`.
    #[dbus_proxy(property, name = "Id")]
    fn id(&self) -> crate::zbus::Result<u32>;

    /// Get property `Unit`.
    #[dbus_proxy(property, name = "Unit")]
    fn unit(&self) -> crate::zbus::Result<(String, zbus::zvariant::OwnedObjectPath)>;

    /// Get property `JobType`.
    #[dbus_proxy(property, name = "JobType")]
    fn job_type(&self) -> crate::zbus::Result<String>;

    /// Get property `State`.
    #[dbus_proxy(property, name = "State")]
    fn state(&self) -> crate::zbus::Result<String>;

    /// Get property `ActivationDetails`.
    #[dbus_proxy(property, name = "ActivationDetails")]
    fn activation_details(&self) -> crate::zbus::Result<Vec<(String, String)>>;
}
