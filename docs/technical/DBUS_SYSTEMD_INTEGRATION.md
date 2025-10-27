# FreezR D-Bus и Systemd Интеграция

FreezR использует **D-Bus API** для управления systemd службами без использования sudo, что обеспечивает безопасность и работу с `NoNewPrivileges=true`.

## 🔒 Архитектура безопасности

### Без sudo - только Linux capabilities и D-Bus

FreezR **НЕ использует sudo** для управления процессами и службами. Вместо этого используется:

1. **Linux Capabilities** - для управления процессами:
   - `CAP_KILL` - отправка сигналов (SIGTERM, SIGKILL, SIGSTOP, SIGCONT)
   - `CAP_SYS_NICE` - изменение приоритета процессов (nice)

2. **D-Bus API** - для управления systemd:
   - `org.freedesktop.systemd1.Manager.RestartUnit` - перезапуск служб
   - `org.freedesktop.systemd1.Manager.Reload` - daemon-reload

3. **Polkit** - авторизация D-Bus операций для пользователя

### Функции без sudo

| Функция | Метод | Требования |
|---------|-------|------------|
| Kill процесса | `nix::sys::signal::kill(SIGTERM/SIGKILL)` | CAP_KILL |
| Freeze процесса | `nix::sys::signal::kill(SIGSTOP)` | CAP_KILL |
| Unfreeze процесса | `nix::sys::signal::kill(SIGCONT)` | CAP_KILL |
| Nice процесса | `libc::setpriority()` | CAP_SYS_NICE |
| Restart службы | D-Bus `RestartUnit` | Polkit rule |
| Daemon reload | D-Bus `Reload` | Polkit rule |

## 📦 Установка и настройка

### 1. Systemd Service файл

**Расположение:** `/etc/systemd/system/freezr.service`

```ini
[Unit]
Description=FreezR Process Monitor - Advanced Resource Management
Documentation=https://github.com/yourusername/freezr
After=network.target multi-user.target

[Service]
Type=simple
User=ryazanov
Group=ryazanov
WorkingDirectory=/home/ryazanov/.myBashScripts/freezr

# Main process
ExecStart=/home/ryazanov/.myBashScripts/freezr/target/release/process-monitor --config /home/ryazanov/.myBashScripts/freezr/freezr.toml --stats --report-interval 60

# Restart policy
Restart=always
RestartSec=10
KillMode=mixed
TimeoutStopSec=30

# Resource limits for the monitor itself
CPUQuota=5%
MemoryMax=50M
MemoryHigh=40M

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=freezr

# Security hardening
NoNewPrivileges=true        # Prevents sudo - работает с D-Bus!
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/ryazanov/.myBashScripts/freezr/logs
ReadWritePaths=/home/ryazanov/.myBashScripts/freezr/data

# Process capabilities (needed for nice, freeze, kill)
AmbientCapabilities=CAP_SYS_NICE CAP_KILL
CapabilityBoundingSet=CAP_SYS_NICE CAP_KILL CAP_DAC_OVERRIDE

[Install]
WantedBy=multi-user.target
```

**Ключевые моменты:**
- ✅ `NoNewPrivileges=true` - запрещает повышение привилегий (sudo не работает)
- ✅ `AmbientCapabilities` - даёт процессу нужные capabilities
- ✅ Работает от обычного пользователя (`User=ryazanov`)

### 2. Polkit правила

**Файл:** `/etc/polkit-1/rules.d/50-freezr-kesl-restart.rules`

```javascript
// Allow FreezR service to manage kesl service via systemd D-Bus
polkit.addRule(function(action, subject) {
    // Allow systemd operations for user ryazanov
    if (action.id == "org.freedesktop.systemd1.manage-units") {
        // Check if subject is user ryazanov
        if (subject.user == "ryazanov") {
            var verb = action.lookup("verb");
            var unit = action.lookup("unit");

            // Allow daemon-reload for all units
            if (verb == "reload") {
                polkit.log("Allowing daemon-reload for user: " + subject.user);
                return polkit.Result.YES;
            }

            // Allow restart specifically for kesl.service
            if (verb == "restart" && unit == "kesl.service") {
                polkit.log("Allowing restart of kesl.service for user: " + subject.user);
                return polkit.Result.YES;
            }
        }
    }
});

// Alternative: Allow all systemd operations for ryazanov (less secure, for testing)
polkit.addRule(function(action, subject) {
    if (action.id.indexOf("org.freedesktop.systemd1") == 0 && subject.user == "ryazanov") {
        return polkit.Result.YES;
    }
});
```

**Установка:**
```bash
sudo cp /tmp/freezr-polkit-rule-fixed.rules /etc/polkit-1/rules.d/50-freezr-kesl-restart.rules
sudo systemctl restart polkit
```

### 3. D-Bus Policy (опционально)

**Файл:** `/etc/dbus-1/system.d/freezr-systemd.conf`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <!-- Allow user ryazanov to manage systemd units -->
  <policy user="ryazanov">
    <allow send_destination="org.freedesktop.systemd1"
           send_interface="org.freedesktop.systemd1.Manager"
           send_member="Reload"/>
    <allow send_destination="org.freedesktop.systemd1"
           send_interface="org.freedesktop.systemd1.Manager"
           send_member="RestartUnit"/>
    <allow send_destination="org.freedesktop.systemd1"
           send_interface="org.freedesktop.systemd1.Manager"
           send_member="GetUnit"/>
    <allow send_destination="org.freedesktop.systemd1"
           send_interface="org.freedesktop.DBus.Properties"
           send_member="Get"/>
    <allow send_destination="org.freedesktop.systemd1"
           send_interface="org.freedesktop.DBus.Properties"
           send_member="GetAll"/>
  </policy>
</busconfig>
```

**Установка:**
```bash
sudo cp /tmp/freezr-dbus-policy.conf /etc/dbus-1/system.d/freezr-systemd.conf
sudo systemctl reload dbus
```

## 🔧 Код D-Bus интеграции

### systemd.rs - D-Bus вместо sudo

```rust
use zbus::{blocking::Connection, zvariant::OwnedObjectPath};

/// Restart the systemd service via D-Bus
fn restart_service(&self) -> Result<()> {
    let proxy = Self::get_manager_proxy()?;

    // Convert service name to systemd unit
    let unit_name = format!("{}.service", self.service_name);

    // Call RestartUnit method via D-Bus
    let _job_path: OwnedObjectPath = proxy
        .call_method("RestartUnit", &(unit_name.as_str(), "replace"))
        .map_err(|e| Error::Systemd(format!("restart {} failed: {}", self.service_name, e)))?
        .body()
        .deserialize()?;

    Ok(())
}
```

**Преимущества:**
- ✅ Не требует sudo
- ✅ Работает с `NoNewPrivileges=true`
- ✅ Контролируется через polkit
- ✅ Стандартный способ для systemd

### executor.rs - Прямые системные вызовы

```rust
/// Kill process - прямой системный вызов
pub fn kill_process(pid: u32) -> Result<()> {
    let process_pid = Pid::from_raw(pid as i32);

    // SIGTERM (graceful)
    kill(process_pid, Signal::SIGTERM)?;
    thread::sleep(Duration::from_secs(2));

    // SIGKILL (force) если нужно
    if Self::process_exists(pid)? {
        kill(process_pid, Signal::SIGKILL)?;
    }

    Ok(())
}

/// Freeze process - SIGSTOP
pub fn freeze_process(pid: u32) -> Result<()> {
    let process_pid = Pid::from_raw(pid as i32);
    kill(process_pid, Signal::SIGSTOP)?;
    Ok(())
}

/// Nice process - прямой setpriority()
pub fn renice_process(pid: u32, nice_level: i32) -> Result<()> {
    let result = unsafe {
        libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, nice_level as libc::c_int)
    };

    if result == -1 {
        return Err(Error::Executor("Failed to set priority".into()));
    }

    Ok(())
}
```

## 🧪 Тестирование

### Проверка capabilities

```bash
# Проверить capabilities процесса
cat /proc/$(pgrep process-monitor)/status | grep Cap

# Декодировать capabilities
capsh --decode=00000000a80425fb
```

### Проверка D-Bus

```bash
# Тест перезапуска KESL через D-Bus (как пользователь ryazanov)
busctl --user call \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  RestartUnit ss "kesl.service" "replace"
```

### Проверка polkit

```bash
# Проверить polkit правила
pkaction --action-id org.freedesktop.systemd1.manage-units --verbose

# Проверить может ли пользователь выполнять действия
pkcheck --action-id org.freedesktop.systemd1.manage-units --process $$ -u
```

## 📊 Мониторинг

### Логи D-Bus операций

```bash
# Логи FreezR с попытками D-Bus
sudo journalctl -u freezr -f | grep -E "(daemon-reload|RestartUnit|D-Bus)"

# Логи polkit авторизации
sudo journalctl -u polkit -f
```

### Проверка работы

```bash
# 1. Убедиться что служба запущена
systemctl status freezr

# 2. Проверить что KESL мониторится
tail -f /home/ryazanov/.myBashScripts/freezr/logs/process_monitor.log.$(date +%Y-%m-%d)

# 3. Дождаться нарушения лимитов KESL
# Должны увидеть:
# [INFO] Restarting KESL service with daemon-reload
# [INFO] KESL service successfully restarted
```

## 🐛 Troubleshooting

### Ошибка: Interactive authentication required

**Проблема:**
```
org.freedesktop.DBus.Error.InteractiveAuthorizationRequired: Interactive authentication required
```

**Решение:**
1. Проверить polkit правила установлены
2. Перезапустить polkit: `sudo systemctl restart polkit`
3. Проверить D-Bus policy установлен
4. Перезагрузить D-Bus: `sudo systemctl reload dbus`

### Ошибка: Permission denied при kill/nice

**Проблема:** Process operation failed with EPERM

**Решение:**
1. Проверить capabilities: `grep Cap /proc/$(pgrep process-monitor)/status`
2. Убедиться что в service файле есть `AmbientCapabilities=CAP_SYS_NICE CAP_KILL`
3. Перезапустить службу: `sudo systemctl restart freezr`

### NoNewPrivileges блокирует sudo

**Это нормально!** FreezR специально не использует sudo.

Если видите ошибку:
```
sudo: The "no new privileges" flag is set
```

Это значит код пытается использовать sudo (старая версия). Обновите до последней версии с D-Bus поддержкой.

## 📚 Ссылки

- [systemd D-Bus API](https://www.freedesktop.org/wiki/Software/systemd/dbus/)
- [Linux Capabilities](https://man7.org/linux/man-pages/man7/capabilities.7.html)
- [Polkit Documentation](https://www.freedesktop.org/software/polkit/docs/latest/)
- [zbus Rust crate](https://docs.rs/zbus/)
