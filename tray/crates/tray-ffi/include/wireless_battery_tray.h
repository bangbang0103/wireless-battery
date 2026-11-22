#ifndef WIRELESS_BATTERY_TRAY_H
#define WIRELESS_BATTERY_TRAY_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque handle owning the background tray service and polling thread. */
typedef struct WbTrayHandle WbTrayHandle;

/**
 * Callback invoked from the background poller thread whenever a new snapshot
 * or low-battery alert payload is available.
 *
 * `update_json_utf8` is a NUL-terminated UTF-8 JSON object:
 * `{ "snapshot": { ... }, "alerts": [ ... ] }`
 * The pointer is valid only for the duration of the callback; copy if needed.
 */
typedef void (*WbTrayUpdateCallback)(const char *update_json_utf8, void *user_data);

/**
 * Create and start the tray service and background poller.
 * Returns NULL if the underlying HID/OS transport fails to initialize.
 */
WbTrayHandle *wb_tray_create(WbTrayUpdateCallback callback, void *user_data);

/** Stop background polling and free the handle. Safe to call with NULL. */
void wb_tray_destroy(WbTrayHandle *handle);

/**
 * Return the latest snapshot as a newly allocated UTF-8 JSON string.
 * Caller must release the returned pointer with `wb_tray_string_free`.
 */
char *wb_tray_snapshot_json(const WbTrayHandle *handle);

/** Trigger an asynchronous full refresh of all eligible devices. */
void wb_tray_request_refresh(const WbTrayHandle *handle);

/** Trigger an asynchronous refresh of a single device by opaque session ID. */
void wb_tray_request_device_refresh(const WbTrayHandle *handle, const char *device_id_utf8);

/**
 * Replace and persist configuration from a UTF-8 JSON `TrayConfig` payload.
 * Returns true if the JSON was valid and applied.
 */
bool wb_tray_update_config_json(const WbTrayHandle *handle, const char *config_json_utf8);

/**
 * Return sanitized passive diagnostic JSON for `device_id_utf8`, or NULL on error.
 * Caller must release non-NULL pointers with `wb_tray_string_free`.
 */
char *wb_tray_diagnose_json(const WbTrayHandle *handle, const char *device_id_utf8);

/**
 * Return sanitized error report JSON (combining the latest read error kind and
 * passive diagnostic metadata) for `device_id_utf8`, or NULL on error.
 * Caller must release non-NULL pointers with `wb_tray_string_free`.
 */
char *wb_tray_error_report_json(const WbTrayHandle *handle, const char *device_id_utf8);

/** Release any UTF-8 string allocated by this library. Safe to call with NULL. */
void wb_tray_string_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* WIRELESS_BATTERY_TRAY_H */
