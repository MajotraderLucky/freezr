# FreezR Systemd Service

FreezR поддерживает установку как systemd сервис для автоматического запуска при загрузке системы.

## 🚀 Быстрый старт

### Установка сервиса

```bash
cd /home/ryazanov/.myBashScripts/freezr
sudo ./target/release/process-monitor install-service
```

Эта команда:
- ✅ Создаст systemd service файл в `/etc/systemd/system/freezr.service`
- ✅ Включит автозапуск при загрузке системы (`systemctl enable`)
- ✅ Запустит сервис немедленно (`systemctl start`)
- ✅ Проверит что сервис успешно запущен

### Проверка статуса

```bash
# Через FreezR CLI
./target/release/process-monitor service-status

# Или напрямую через systemctl
sudo systemctl status freezr
```

### Удаление сервиса

```bash
sudo ./target/release/process-monitor uninstall-service
```

## 📋 Команды управления

### FreezR CLI команды

```bash
# Установить (с подтверждением)
sudo ./target/release/process-monitor install-service

# Установить (без подтверждения)
sudo ./target/release/process-monitor install-service --yes

# Проверить статус
./target/release/process-monitor service-status

# Удалить
sudo ./target/release/process-monitor uninstall-service

# Удалить (без подтверждения)
sudo ./target/release/process-monitor uninstall-service --yes
```

### Systemd команды

```bash
# Статус
sudo systemctl status freezr

# Запустить
sudo systemctl start freezr

# Остановить
sudo systemctl stop freezr

# Перезапустить
sudo systemctl restart freezr

# Включить автозапуск
sudo systemctl enable freezr

# Выключить автозапуск
sudo systemctl disable freezr
```

### Просмотр логов

```bash
# Real-time логи
sudo journalctl -u freezr -f

# Последние 50 строк
sudo journalctl -u freezr -n 50

# Логи с определенного времени
sudo journalctl -u freezr --since "1 hour ago"

# Логи за сегодня
sudo journalctl -u freezr --since today
```

## ⚙️ Конфигурация сервиса

Сервис автоматически настраивается со следующими параметрами:

### Основные настройки
- **User**: Ваш текущий пользователь (не root!)
- **WorkingDirectory**: `/home/ryazanov/.myBashScripts/freezr`
- **Config**: `/home/ryazanov/.myBashScripts/freezr/freezr.toml`

### Мониторинг
- **Все процессы**: KESL, Node.js, Snap, Firefox, Brave, Telegram
- **Memory Pressure**: PSI-based OOM prevention ✅
- **Statistics**: Extended stats с 60-секундными отчетами
- **Logging**: Journald (systemd logs)

### Автоматический перезапуск
- **Restart**: always
- **RestartSec**: 10 секунд
- **Auto-start on boot**: enabled

### Ограничения ресурсов
Для самого монитора (чтобы не потреблял много):
- **CPU**: 5% (CPUQuota=5%)
- **Memory**: 50MB max (MemoryMax=50M)
- **Memory High**: 40MB (MemoryHigh=40M)

### Безопасность
- **NoNewPrivileges**: true
- **PrivateTmp**: true
- **ProtectSystem**: strict
- **ProtectHome**: read-only
- **Capabilities**: CAP_SYS_NICE, CAP_KILL (только необходимые)

## 🔍 Диагностика

### Проверка что сервис работает

```bash
# Статус сервиса
sudo systemctl is-active freezr
# Ожидается: active

# Полный статус
sudo systemctl status freezr

# Последние логи
sudo journalctl -u freezr -n 20
```

### Проблемы при установке

**Ошибка: "This command must be run with sudo"**
```bash
# Решение: добавьте sudo
sudo ./target/release/process-monitor install-service
```

**Ошибка: "Failed to write service file"**
```bash
# Проверьте права доступа
ls -la /etc/systemd/system/

# Убедитесь что запущено с sudo
whoami  # должно быть root при использовании sudo
```

### Сервис не запускается

```bash
# Проверьте логи ошибок
sudo journalctl -u freezr -n 50 --no-pager

# Проверьте конфигурацию
cat /home/ryazanov/.myBashScripts/freezr/freezr.toml

# Проверьте что бинарник существует
ls -lh /home/ryazanov/.myBashScripts/freezr/target/release/process-monitor
```

### Переустановка сервиса

```bash
# Удалить старый
sudo ./target/release/process-monitor uninstall-service --yes

# Установить заново
sudo ./target/release/process-monitor install-service --yes
```

## 📊 Мониторинг работы

### Проверка что мониторинг работает

```bash
# Логи в реальном времени
sudo journalctl -u freezr -f

# Вы должны увидеть:
# - "🦀 Process Monitor starting..."
# - "Memory pressure monitoring enabled"
# - "KESL: CPU X.X%, Memory XXmb"
# - Периодические отчеты каждые 60 секунд
```

### Проверка действий при нагрузке

Логи будут показывать когда FreezR выполняет действия:

```
# Memory pressure warning
[WARN] WARNING memory pressure detected! some=12.50%, full=0.00%

# Firefox freeze
[INFO] Froze Firefox process 12345

# KESL restart
[INFO] KESL успешно перезапущен с применением лимитов
```

## 🧪 Тестирование

Используйте тестовый скрипт:

```bash
cd /home/ryazanov/.myBashScripts/freezr
./test-service-install.sh
```

Этот скрипт:
1. Проверит текущий статус
2. Установит сервис (попросит sudo пароль)
3. Проверит что установка успешна
4. Покажет последние логи
5. Выведет полезные команды

## 🎯 Production Ready

FreezR systemd сервис готов к production использованию:

✅ **Автоматический запуск** при загрузке системы
✅ **Автоперезапуск** при сбоях
✅ **Ограничения ресурсов** для самого монитора
✅ **Безопасная конфигурация** (не root, minimal capabilities)
✅ **Централизованные логи** через journald
✅ **Полный мониторинг** всех настроенных процессов
✅ **Memory Pressure** защита от OOM

---

**Последнее обновление**: 2025-10-26
**Статус**: Production Ready ✅
