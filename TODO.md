### **Checklist del Proyecto: Implementación de `mypthreads` con Cambio de Contexto**

Esta guía te ayudará a reestructurar tu proyecto, pasando de un modelo de simulación a una biblioteca de hilos preemptivos a nivel de usuario.

---

### **Fase 1: El Mecanismo de Cambio de Contexto (La Base)**
*Objetivo: Crear la maquinaria de bajo nivel para guardar y restaurar el estado de un hilo.*

-   [X] **Definir la estructura `ThreadContext`:**
    -   Crea una struct que contenga los registros del CPU que se deben preservar entre cambios de contexto (mínimo: `rsp`, `rbp`, `rbx`, `r12`, `r13`, `r14`, `r15`, y un espacio para `rip`).
    -    reality: Usamos el crate `context` con `ProtectedFixedSizeStack`, Encapsulamos `Context` y `Stack` en nuestra struct.

-   [X] **Implementar la función `switch`:**
    -   Escribe una función `unsafe`, probablemente usando `core::arch::asm!`, que tome dos punteros a `ThreadContext` (el viejo y el nuevo).
    -   Esta función debe guardar los registros del hilo actual en el contexto viejo y cargar los registros desde el contexto nuevo.
    -   reality: Implementamos `resume()` que hace el cambio de contexto, Usa el crate `context` (no escribimos ensamblador manual).

-   [X] **Modificar la estructura `MyThread`:**
    -   Añade un campo para el contexto: `context: ThreadContext`.
    -   Añade un campo para la pila: `stack: Vec<u8>`.
    - reality: thread_v2.rs: MyThreadV2 incluye context: ThreadContext y entry: ContextThreadEntry

-   [X] **Implementar la Asignación e Inicialización de la Pila:**
    -   En `my_thread_create`, asigna un `Vec<u8>` de un tamaño fijo (ej. 8KB) para que sea la pila del nuevo hilo.
    -   Crea una función `thread_entry_wrapper` que se encargue de llamar a la función de entrada del hilo y, a su término, a `my_thread_end`.
    -   Prepara la pila del nuevo hilo para que, cuando se cambie a su contexto por primera vez, el puntero de instrucción (`rip`) apunte a `thread_entry_wrapper`.
    -    reality: thread_v2.rs: se crea con ThreadContext::new(thread_entry_wrapper); el wrapper desempaqueta TransferMessage::Init, inicializa TLS (ThreadGlobalContext/api_context), ejecuta el closure y devuelve ThreadResponse al runtime

---

### **Fase 2: La Lógica del Runtime y el Planificador**
*Objetivo: Construir el "director de orquesta" que utiliza el cambio de contexto para gestionar los hilos.*

-   [X] **Adaptar `ThreadRuntime`:**
    -   Añade un campo para el contexto principal del planificador: `runtime_context: ThreadContext`. Esto es crucial para que los hilos puedan "volver" al planificador.

-   [X] **Reescribir el Bucle de Ejecución:**
    -   Reemplaza la función `run_once` con un bucle principal (ej. `run_scheduler`).
    -   Dentro del bucle:
        1.  Selecciona el siguiente hilo a ejecutar desde la cola `ready`.
        2.  Llama a `switch(&mut runtime_context, &new_thread.context)` para transferir el control.
    -  reality: Hay run_once() y run(cycles), extrae tid de ready, hace resume_with_data(Init/Continue) y procesa ThreadResponse (Yield/Block/Exit). ✔️
    -   **Pendiente**: función única de selección que aplique RR/Lottery/RT (hoy es FIFO de VecDeque)

-   [X] **Crear un `main` de Prueba Mínimo:**
    -   Crea un programa de prueba que no use la lógica de `ThreadCity` todavía.
    -   El objetivo es simple: crear dos hilos que se impriman un mensaje y se llamen `my_thread_yield()` mutuamente un par de veces. Si esto funciona, la base está lista.

---

### **Fase 3: Implementar las Primitivas de los Hilos**
*Objetivo: Desarrollar la API `my_thread_*` que los hilos usarán para interactuar con el planificador.*

-   [X] **Implementar `my_thread_yield()`:**
    -   Su única función es transferir el control de vuelta al planificador. Debe llamar a `switch(&mut current_thread.context, &runtime.runtime_context)`.
    -  reality: Hilos devuelven ThreadSignal::Yield (vía API de contexto) y el wrapper lo traduce a ThreadResponse::Yield so runtime reencola

-   [X] **Implementar `my_thread_end()`:**
    -   Marca el estado del hilo actual como `Terminated`.
    -   Despierta a cualquier hilo que estuviera esperando en `join` (mueve sus estados a `Ready`).
    -   Llama a `switch` para volver permanentemente al planificador.

-   [ X] **Implementar `my_thread_join()`:**
    -   Si el hilo objetivo no ha terminado:
        1.  Añade el hilo actual a la lista `joiners` del hilo objetivo.
        2.  Cambia el estado del hilo actual a `Blocked`.
        3.  Llama a `my_thread_yield()` para ceder el control.

---

### **Fase 4: Implementar Primitivas de Sincronización (Mutex)**
*Objetivo: Utilizar el sistema de bloqueo para crear mutex funcionales.*

-   [X] **Implementar `my_mutex_lock()`:**
    -   Si el mutex está libre, tómalo y retorna.
    -   Si está ocupado:
        1.  Añade el hilo actual a la `wait_queue` del mutex.
        2.  Cambia el estado del hilo a `Blocked`.
        3.  Llama a `my_thread_yield()` para ceder el procesador.

-   [X] **Implementar `my_mutex_unlock()`:**
    -   Libera el mutex.
    -   Si la `wait_queue` no está vacía, saca el siguiente hilo y cambia su estado de `Blocked` a `Ready` para que el planificador pueda ejecutarlo.

---

### **Fase 5: Finalización, Limpieza y Schedulers Avanzados**
*Objetivo: Completar las funcionalidades requeridas y limpiar la memoria.*

-   [X] **Implementar `my_thread_detach()`:**
    -   Marca un hilo para que sus recursos se liberen automáticamente al terminar.

-   [ ] **Asegurar la Liberación de Memoria:**
    -   Verifica que la memoria de la pila (`stack: Vec<u8>`) de un hilo se libere correctamente cuando este termina (ya sea por `join` o por estar `detached`).

-   [ ] **Integrar Schedulers Avanzados:**
    -   Adapta tus funciones `schedule_lottery` y `schedule_realtime` para que funcionen con el nuevo modelo dentro de la función `select_next_thread` del planificador.

-   [ ] **Integrar con `ThreadCity`:**
    -   Una vez que toda la biblioteca `mypthreads` sea funcional y estable, conéctala a la lógica de tu simulación `ThreadCity`.
