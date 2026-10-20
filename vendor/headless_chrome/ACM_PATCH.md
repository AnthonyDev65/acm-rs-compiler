headless_chrome 1.0.22 sin modificar salvo READ_TIMEOUT_DURATION (src/browser/transport/web_socket_connection.rs): 100 ms -> 2 ms.
El hilo lector retiene el mutex del socket durante cada read con timeout, así que cada envío esperaba hasta 100 ms.
