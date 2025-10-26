#!/bin/bash
# FreezR Log Maintenance Script
# Архивация, сжатие и удаление старых логов

set -e

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Конфигурация
LOG_DIR="logs"
ARCHIVE_DIR="logs/archive"
DAYS_TO_KEEP_ACTIVE=7        # Сколько дней хранить активные логи
DAYS_TO_KEEP_ARCHIVE=30      # Сколько дней хранить сжатые архивы

# Функция для вывода с временной меткой
log_info() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] INFO:${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARN:${NC} $1"
}

log_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

# Проверка директорий
check_directories() {
    log_info "Проверка директорий..."

    if [ ! -d "$LOG_DIR" ]; then
        log_error "Директория $LOG_DIR не найдена!"
        exit 1
    fi

    if [ ! -d "$ARCHIVE_DIR" ]; then
        log_warn "Директория $ARCHIVE_DIR не найдена, создаём..."
        mkdir -p "$ARCHIVE_DIR"
    fi

    log_info "✓ Директории проверены"
}

# Показать статистику
show_stats() {
    log_info "📊 Статистика логов:"

    local active_logs=$(find "$LOG_DIR" -maxdepth 1 -name "*.log.*" -type f 2>/dev/null | wc -l)
    local active_size=$(du -sh "$LOG_DIR" 2>/dev/null | cut -f1)
    local archive_logs=$(find "$ARCHIVE_DIR" -name "*.gz" -type f 2>/dev/null | wc -l)
    local archive_size=$(du -sh "$ARCHIVE_DIR" 2>/dev/null | cut -f1)

    echo "  Активные логи: $active_logs файлов ($active_size)"
    echo "  Архивные логи: $archive_logs файлов ($archive_size)"
}

# Архивировать старые логи (старше DAYS_TO_KEEP_ACTIVE дней)
archive_old_logs() {
    log_info "🗜️  Архивация старых логов (старше $DAYS_TO_KEEP_ACTIVE дней)..."

    local count=0
    local saved_space=0

    # Найти логи старше N дней
    while IFS= read -r -d '' logfile; do
        if [ -f "$logfile" ]; then
            local filename=$(basename "$logfile")
            local archive_name="${filename}.gz"
            local archive_path="$ARCHIVE_DIR/$archive_name"

            # Получить размер до сжатия
            local before_size=$(stat -f%z "$logfile" 2>/dev/null || stat -c%s "$logfile")

            # Сжать и переместить
            if gzip -c "$logfile" > "$archive_path"; then
                local after_size=$(stat -f%z "$archive_path" 2>/dev/null || stat -c%s "$archive_path")
                local saved=$((before_size - after_size))
                saved_space=$((saved_space + saved))

                rm "$logfile"
                count=$((count + 1))

                log_info "  ✓ $filename → $archive_name (сэкономлено: $(numfmt --to=iec $saved 2>/dev/null || echo "$saved bytes"))"
            else
                log_error "  ✗ Не удалось сжать $filename"
            fi
        fi
    done < <(find "$LOG_DIR" -maxdepth 1 -name "*.log.*" -type f -mtime +$DAYS_TO_KEEP_ACTIVE -print0)

    if [ $count -eq 0 ]; then
        log_info "  Нет логов для архивации"
    else
        log_info "  ✓ Заархивировано: $count файлов"
        log_info "  ✓ Сэкономлено места: $(numfmt --to=iec $saved_space 2>/dev/null || echo "$saved_space bytes")"
    fi
}

# Удалить очень старые архивы (старше DAYS_TO_KEEP_ARCHIVE дней)
delete_old_archives() {
    log_info "🗑️  Удаление старых архивов (старше $DAYS_TO_KEEP_ARCHIVE дней)..."

    local count=0
    local freed_space=0

    # Найти архивы старше N дней
    while IFS= read -r -d '' archive; do
        if [ -f "$archive" ]; then
            local filename=$(basename "$archive")
            local size=$(stat -f%z "$archive" 2>/dev/null || stat -c%s "$archive")

            freed_space=$((freed_space + size))
            rm "$archive"
            count=$((count + 1))

            log_info "  ✓ Удалён: $filename ($(numfmt --to=iec $size 2>/dev/null || echo "$size bytes"))"
        fi
    done < <(find "$ARCHIVE_DIR" -name "*.gz" -type f -mtime +$DAYS_TO_KEEP_ARCHIVE -print0)

    if [ $count -eq 0 ]; then
        log_info "  Нет архивов для удаления"
    else
        log_info "  ✓ Удалено: $count архивов"
        log_info "  ✓ Освобождено места: $(numfmt --to=iec $freed_space 2>/dev/null || echo "$freed_space bytes")"
    fi
}

# Показать список файлов для проверки
list_files() {
    log_info "📝 Список логов:"

    echo ""
    echo "Активные логи (logs/):"
    find "$LOG_DIR" -maxdepth 1 -name "*.log.*" -type f -exec ls -lh {} \; 2>/dev/null | awk '{print "  " $9 " - " $5 " (" $6 " " $7 ")"}'

    echo ""
    echo "Архивные логи (logs/archive/):"
    find "$ARCHIVE_DIR" -name "*.gz" -type f -exec ls -lh {} \; 2>/dev/null | awk '{print "  " $9 " - " $5 " (" $6 " " $7 ")"}'
}

# Основная функция
main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║          FreezR Log Maintenance                           ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo ""

    check_directories
    echo ""

    show_stats
    echo ""

    case "${1:-full}" in
        "archive")
            archive_old_logs
            ;;
        "clean")
            delete_old_archives
            ;;
        "list")
            list_files
            ;;
        "stats")
            # Уже показали выше
            ;;
        "full"|*)
            archive_old_logs
            echo ""
            delete_old_archives
            echo ""
            show_stats
            ;;
    esac

    echo ""
    log_info "✅ Обслуживание логов завершено!"
    echo ""
}

# Справка
if [ "$1" == "help" ] || [ "$1" == "-h" ] || [ "$1" == "--help" ]; then
    cat <<EOF
FreezR Log Maintenance Script

Использование:
  $0 [команда]

Команды:
  full      - Полное обслуживание (архивация + очистка) [по умолчанию]
  archive   - Только архивация старых логов
  clean     - Только удаление старых архивов
  list      - Показать список всех логов
  stats     - Показать статистику
  help      - Показать эту справку

Конфигурация:
  - Активные логи хранятся: $DAYS_TO_KEEP_ACTIVE дней
  - Архивные логи хранятся: $DAYS_TO_KEEP_ARCHIVE дней
  - Директория логов: $LOG_DIR
  - Директория архива: $ARCHIVE_DIR

Примеры:
  $0                 # Полное обслуживание
  $0 archive         # Только архивировать старые логи
  $0 clean           # Только удалить старые архивы
  $0 list            # Показать список логов

EOF
    exit 0
fi

# Запуск
main "$@"
