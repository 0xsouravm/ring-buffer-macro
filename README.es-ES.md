# ring-buffer-macro

[![Crates.io](https://img.shields.io/crates/v/ring-buffer-macro?style=flat-square)](https://crates.io/crates/ring-buffer-macro)
[![docs.rs](https://img.shields.io/docsrs/ring-buffer-macro?style=flat-square)](https://docs.rs/ring-buffer-macro)
[![License](https://img.shields.io/crates/l/ring-buffer-macro?style=flat-square)](LICENSE)
[![Website](https://img.shields.io/badge/website-ringbuf.dev-F97316?style=flat-square)](https://ringbuf.dev)

Una macro de procedimiento (proc macro) que convierte un struct de tupla en un ring buffer de tamaño fijo. Soporta los modos single-threaded, lock-free SPSC y lock-free MPSC.

```toml
[dependencies]
ring-buffer-macro = "0.2.0"
```

## Cómo funciona

Escribe un struct de tupla con un campo que indique el tipo de elemento:

```rust
#[ring_buffer(5)]
struct IntBuffer(i32);
```

La macro reemplaza esto con un struct nombrado (`data`, `head`, `tail`, etc.) y genera un bloque `impl` completo con los métodos de ring buffer. El struct de tupla es solo una forma de especificar el nombre y el tipo de elemento; el campo `(i32)` se descarta.

Para los modos concurrentes (SPSC/MPSC), el struct generado utiliza `UnsafeCell<Vec<MaybeUninit<T>>>` con índices atómicos en lugar de un `Vec<T>` simple, por lo que los elementos se mueven en lugar de clonarse. Esto reduce el límite del trait de `Clone` a `Send`.

## Uso

### Modo estándar

```rust
use ring_buffer_macro::ring_buffer;

#[ring_buffer(5)]
struct IntBuffer(i32);

fn main() {
    let mut buf = IntBuffer::new();

    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();

    assert_eq!(buf.peek(), Some(&1));
    assert_eq!(buf.peek_back(), Some(&3));

    for item in buf.iter() {
        println!("{}", item);
    }

    assert_eq!(buf.dequeue(), Some(1));

    // drain() elimina los elementos mientras itera
    let rest: Vec<_> = buf.drain().collect();
    assert!(buf.is_empty());
}
```

### Genéricos

Los parámetros de tipo se preservan en el código generado:

```rust
#[ring_buffer(10)]
struct GenericBuffer<T: Clone>(T);

let mut buf: GenericBuffer<String> = GenericBuffer::new();
buf.enqueue("hello".to_string()).unwrap();
```

### SPSC (lock-free, productor único/consumidor único)

Utiliza `AtomicUsize` para head/tail con ordenamiento acquire/release. Sin bloqueos (locks).

```rust
use ring_buffer_macro::ring_buffer;
use std::sync::Arc;
use std::thread;

#[ring_buffer(capacity = 1024, mode = "spsc")]
struct MessageQueue(String);

fn main() {
    let queue = Arc::new(MessageQueue::new());

    let q1 = Arc::clone(&queue);
    let producer = thread::spawn(move || {
        let (p, _) = q1.split();
        for i in 0..100 {
            while p.try_enqueue(format!("msg {}", i)).is_err() {}
        }
    });

    let q2 = Arc::clone(&queue);
    let consumer = thread::spawn(move || {
        let (_, c) = q2.split();
        for _ in 0..100 {
            while c.try_dequeue().is_none() {}
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}
```

### MPSC (múltiples productores, consumidor único)

Los productores se coordinan mediante `compare_exchange_weak` en el índice tail. Cada ranura tiene un flag `AtomicBool` para que el consumidor sepa cuando una escritura ha terminado.

El manejador del productor es `Clone`; el del consumidor no. Solo debe existir un consumidor; esto es una restricción del protocolo, no se impone en tiempo de ejecución.

```rust
use ring_buffer_macro::ring_buffer;
use std::sync::Arc;
use std::thread;

#[ring_buffer(capacity = 1024, mode = "mpsc")]
struct WorkQueue(i32);

fn main() {
    let queue = Arc::new(WorkQueue::new());
    let mut handles = vec![];

    for i in 0..4 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            let producer = q.producer();
            for j in 0..100 {
                while producer.try_enqueue(i * 100 + j).is_err() {}
            }
        }));
    }

    let q = Arc::clone(&queue);
    handles.push(thread::spawn(move || {
        let consumer = q.consumer();
        let mut count = 0;
        while count < 400 {
            if consumer.try_dequeue().is_some() {
                count += 1;
            }
        }
    }));

    for h in handles {
        h.join().unwrap();
    }
}
```

### Modo bloqueante (Blocking mode)

Disponible tanto para SPSC como para MPSC. Utiliza un par `Mutex<()>` + `Condvar`; el mutex no protege los datos, solo satisface la API de condvar. La ruta de datos real sigue utilizando atómicos lock-free.

```rust
use ring_buffer_macro::ring_buffer;

#[ring_buffer(capacity = 64, mode = "mpsc", blocking = true)]
struct BlockingQueue(String);

// Estos esperan en lugar de devolver Err/None
// producer.enqueue_blocking("message".to_string());
// let msg = consumer.dequeue_blocking();
```

### Optimización de potencia de dos

Si la capacidad es una potencia de dos, se sustituye el operador módulo por un AND bit a bit en el ajuste de los índices (wraparound). La macro impone esta restricción en tiempo de compilación.

```rust
#[ring_buffer(capacity = 1024, power_of_two = true)]
struct FastBuffer(u8);
```

### Relleno de línea de caché (Cache-line padding)

Alinea head y tail en límites de 64 bytes para evitar el "false sharing" cuando el productor y el consumidor se ejecutan en núcleos diferentes. Relevante principalmente para el modo SPSC.

```rust
#[ring_buffer(capacity = 1024, mode = "spsc", cache_padded = true)]
struct PaddedQueue(u8);
```

## Configuración

| Opción | Valores | Por defecto | Descripción |
|--------|--------|---------|-------------|
| `capacity` | entero positivo | requerido | Número máximo de elementos |
| `mode` | `"standard"`, `"spsc"`, `"mpsc"` | `"standard"` | Modo del buffer |
| `power_of_two` | `true`, `false` | `false` | Indexación bit a bit (la capacidad debe ser 2^n) |
| `cache_padded` | `true`, `false` | `false` | Alineación de head/tail a 64 bytes para evitar false sharing |
| `blocking` | `true`, `false` | `false` | Enqueue/dequeue bloqueante (solo modos concurrentes) |

```rust
// simple
#[ring_buffer(10)]

// nombrada
#[ring_buffer(capacity = 1024, mode = "spsc", power_of_two = true, cache_padded = true)]
```

## API Generada

### Modo estándar

- `new()` / `enqueue(item)` / `dequeue()` / `clear()`
- `peek()` / `peek_mut()` / `peek_back()`
- `iter()` / `drain()`
- `is_full()` / `is_empty()` / `len()` / `capacity()`

`dequeue()` y `drain()` requieren `T: Clone` (el límite está en el método, no en el struct, por lo que puedes crear un buffer de tipos no-Clone, pero no podrás hacer dequeue de él).

### Modo SPSC

Buffer: `new()`, `split() -> (Producer, Consumer)`, `is_full()`, `is_empty()`, `len()`, `capacity()`

Producer: `try_enqueue(item)`, `enqueue_blocking(item)` (si es blocking)

Consumer: `try_dequeue()`, `dequeue_blocking()` (si es blocking), `peek()`

### Modo MPSC

Buffer: `new()`, `producer()`, `consumer()`, `is_empty()`, `len()`, `capacity()`

Producer (clonable): `try_enqueue(item)`, `enqueue_blocking(item)` (si es blocking), `is_full()`

Consumer: `try_dequeue()`, `dequeue_blocking()` (si es blocking), `peek()`, `is_empty()`, `len()`

## Requisitos

- La entrada debe ser un struct de tupla con un campo: `struct Buffer(i32)`
- El modo estándar requiere `T: Clone` (solo para dequeue/drain)
- Los modos SPSC/MPSC requieren `T: Send`
- La capacidad debe ser un entero positivo

## Licencia

MIT
