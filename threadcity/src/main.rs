
fn main() {
    // 1. Inicializa el runtime global de mypthreads una sola vez al inicio del programa.
    mypthreads::mypthreads_api::runtime_init();

    // 2. Ejecuta la simulación completa, cuya lógica ahora reside en el módulo `runner`.
    threadcity::run_simulation();
}