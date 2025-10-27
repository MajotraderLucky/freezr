# FreezR Security Quick Reference

## ✅ Безопасная архитектура (БЕЗ sudo!)

```
┌─────────────────────────────────────────────────────┐
│  FreezR Process (User: ryazanov)                    │
│  ├─ NoNewPrivileges=true ✓                          │
│  ├─ Capabilities: CAP_KILL, CAP_SYS_NICE ✓         │
│  └─ D-Bus: systemd1.Manager ✓                       │
└─────────────────────────────────────────────────────┘
           │                    │                │
    ───────┴─────────    ───────┴──────    ─────┴─────
   │ Kill/Freeze    │   │ Nice level │   │ D-Bus API │
   │ nix::kill()    │   │ setpriority│   │ RestartUnit│
   └────────────────┘   └────────────┘   └───────────┘
```

## 🔒 Что НЕ используется

```
❌ sudo systemctl restart kesl
❌ sudo renice -n 15 -p 1234
❌ sudo kill -9 1234
❌ SUID binaries
❌ Root user
```

## ✅ Что используется

```
✅ D-Bus: org.freedesktop.systemd1.Manager.RestartUnit
✅ libc::setpriority(PRIO_PROCESS, pid, nice)
✅ nix::sys::signal::kill(pid, SIGTERM)
✅ Linux Capabilities: CAP_KILL, CAP_SYS_NICE
✅ Polkit authorization rules
```

## 🛡️ Capabilities

| Capability | Для чего | Где используется |
|------------|----------|------------------|
| `CAP_KILL` | Сигналы процессам | Kill Node.js, Freeze Firefox/Brave/Telegram |
| `CAP_SYS_NICE` | Изменение приоритета | Nice Snap процессов |

## 🚪 D-Bus + Polkit

**Polkit rule:** `/etc/polkit-1/rules.d/50-freezr-kesl-restart.rules`

```javascript
// Разрешить только:
// - user: ryazanov
// - operation: restart, reload
// - unit: kesl.service
```

**D-Bus policy:** `/etc/dbus-1/system.d/freezr-systemd.conf`

```xml
<!-- Разрешить доступ к systemd1.Manager API -->
```

## 🔍 Проверка безопасности

### 1. Проверить capabilities
```bash
cat /proc/$(pgrep process-monitor)/status | grep Cap
# Ищите: CapEff с CAP_KILL и CAP_SYS_NICE
```

### 2. Проверить NoNewPrivileges
```bash
cat /proc/$(pgrep process-monitor)/status | grep NoNewPrivs
# Должно быть: NoNewPrivs: 1
```

### 3. Проверить что sudo не работает
```bash
# Попытка sudo из FreezR процесса вернёт:
# "sudo: The 'no new privileges' flag is set"
```

### 4. Проверить D-Bus доступ
```bash
# Тест RestartUnit
busctl call \
  org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  RestartUnit ss "kesl.service" "replace"
```

## 📊 Функции и их методы

| Функция | Метод | Файл | Строка |
|---------|-------|------|--------|
| Kill процесса | `nix::kill(SIGTERM/SIGKILL)` | executor.rs | 22-63 |
| Freeze процесса | `nix::kill(SIGSTOP)` | executor.rs | 82-100 |
| Unfreeze | `nix::kill(SIGCONT)` | executor.rs | 102-120 |
| Nice | `libc::setpriority()` | executor.rs | 122-160 |
| Restart KESL | D-Bus `RestartUnit` | systemd.rs | 70-98 |
| Daemon reload | D-Bus `Reload` | systemd.rs | 59-68 |

## 🚨 Troubleshooting

### Ошибка: Interactive authentication required

```bash
# 1. Установить polkit rules
sudo cp freezr-polkit-rule.rules /etc/polkit-1/rules.d/50-freezr-kesl-restart.rules

# 2. Перезапустить polkit
sudo systemctl restart polkit

# 3. Установить D-Bus policy
sudo cp freezr-dbus-policy.conf /etc/dbus-1/system.d/freezr-systemd.conf

# 4. Перезагрузить D-Bus
sudo systemctl reload dbus

# 5. Перезапустить FreezR
sudo systemctl restart freezr
```

### Ошибка: Permission denied при kill/nice

```bash
# Проверить capabilities
grep Cap /proc/$(pgrep process-monitor)/status

# Если не видите CAP_KILL и CAP_SYS_NICE:
# 1. Проверить service файл
grep AmbientCapabilities /etc/systemd/system/freezr.service

# 2. Перезапустить службу
sudo systemctl restart freezr
```

### KESL не перезапускается

```bash
# 1. Проверить polkit логи
sudo journalctl -u polkit | tail -20

# 2. Проверить FreezR логи
sudo journalctl -u freezr | grep -E "(daemon-reload|RestartUnit|D-Bus)"

# 3. Тест D-Bus вручную
busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager RestartUnit ss "kesl.service" "replace"
```

## 📚 Документация

- **Полная документация:** [DBUS_SYSTEMD_INTEGRATION.md](docs/technical/DBUS_SYSTEMD_INTEGRATION.md)
- **Архитектура безопасности:** [SECURITY.md](SECURITY.md)
- **История изменений:** [CHANGELOG_DBUS.md](CHANGELOG_DBUS.md)

## ✅ Чеклист установки

- [ ] Скомпилирован с D-Bus: `cargo build --release`
- [ ] Service файл установлен: `/etc/systemd/system/freezr.service`
- [ ] Polkit rules установлены: `/etc/polkit-1/rules.d/50-freezr-kesl-restart.rules`
- [ ] D-Bus policy установлен: `/etc/dbus-1/system.d/freezr-systemd.conf`
- [ ] Polkit перезапущен: `sudo systemctl restart polkit`
- [ ] D-Bus перезагружен: `sudo systemctl reload dbus`
- [ ] FreezR служба запущена: `sudo systemctl start freezr`
- [ ] Capabilities проверены: `grep Cap /proc/.../status`
- [ ] NoNewPrivileges включен: `grep NoNewPrivs /proc/.../status`
- [ ] D-Bus работает: тест через `busctl`

## 🎯 Принцип наименьших привилегий

```
Root может: ВСЁ (full system access)
         │
         └─> ❌ FreezR НЕ использует root

FreezR может:
  ✅ Kill/freeze процессы пользователя ryazanov
  ✅ Nice процессы пользователя ryazanov
  ✅ Restart kesl.service через D-Bus
  ❌ Kill процессы других пользователей
  ❌ Kill системные процессы
  ❌ Изменять системные файлы
  ❌ Получить root доступ
```

---

**Версия:** v0.1.0
**Дата:** 2025-10-27
**Статус:** ✅ Production Ready
