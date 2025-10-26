#!/bin/bash
# Setup cron job for FreezR log maintenance

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_SCRIPT="$SCRIPT_DIR/log_maintenance.sh"
CRON_LOG="/home/ryazanov/.myBashScripts/freezr/logs/log_maintenance_cron.log"

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║      FreezR Log Maintenance - Cron Setup                  ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Проверка существования скрипта
if [ ! -f "$LOG_SCRIPT" ]; then
    echo "❌ Скрипт $LOG_SCRIPT не найден!"
    exit 1
fi

# Сделать скрипт исполняемым
chmod +x "$LOG_SCRIPT"

# Проверить текущие cron задачи
echo "📋 Проверка текущих cron задач FreezR..."
EXISTING=$(crontab -l 2>/dev/null | grep -c "log_maintenance.sh" || echo "0")

if [ "$EXISTING" -gt 0 ]; then
    echo "⚠️  Найдены существующие задачи ($EXISTING шт.)"
    echo ""
    crontab -l 2>/dev/null | grep "log_maintenance.sh"
    echo ""
    read -p "Удалить существующие и создать новые? (y/n): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Удалить старые задачи
        crontab -l 2>/dev/null | grep -v "log_maintenance.sh" | crontab -
        echo "✓ Старые задачи удалены"
    else
        echo "Отменено"
        exit 0
    fi
fi

# Добавить новые cron задачи
echo ""
echo "📝 Добавление cron задач..."

# Получить текущий crontab
CURRENT_CRON=$(crontab -l 2>/dev/null)

# Создать новый crontab с комментариями и задачами
{
    echo "$CURRENT_CRON"
    echo ""
    echo "# FreezR Log Maintenance"
    echo "# Ежедневное обслуживание логов в 3:00 утра"
    echo "0 3 * * * cd $(dirname $LOG_SCRIPT)/.. && $LOG_SCRIPT full >> $CRON_LOG 2>&1"
    echo ""
    echo "# FreezR Log Maintenance - Weekly detailed report"
    echo "# Еженедельный детальный отчёт в понедельник в 4:00 утра"
    echo "0 4 * * 1 cd $(dirname $LOG_SCRIPT)/.. && $LOG_SCRIPT list >> $CRON_LOG 2>&1"
} | crontab -

echo "✅ Cron задачи добавлены!"
echo ""
echo "📋 Текущие задачи FreezR:"
crontab -l | grep -A2 "FreezR Log Maintenance"
echo ""
echo "📝 Логи cron будут записываться в:"
echo "   $CRON_LOG"
echo ""
echo "🔍 Проверить задачи: crontab -l"
echo "✏️  Редактировать: crontab -e"
echo "🗑️  Удалить все: crontab -r"
echo ""
echo "✅ Настройка завершена!"
