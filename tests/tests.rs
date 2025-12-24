use ring_buffer_macro::ring_buffer;
use std::rc::Rc;
use std::sync::Arc;

#[ring_buffer(5)]
struct TestBuffer {
    data: Vec<i32>,
}

#[ring_buffer(10)]
struct GenericBuffer<T: Clone> {
    data: Vec<T>,
}

#[ring_buffer(5)]
struct BoxBuffer {
    data: Vec<Box<String>>,
}

#[ring_buffer(5)]
struct RcBuffer {
    data: Vec<Rc<String>>,
}

#[ring_buffer(5)]
struct ArcBuffer {
    data: Vec<Arc<String>>,
}

#[ring_buffer(5)]
struct NestedGenericBuffer<T: Clone> {
    data: Vec<Vec<T>>,
}

#[derive(Clone, Debug, PartialEq)]
struct TraitBoundType<T: Clone> {
    value: T,
}

#[ring_buffer(5)]
struct TraitBoundBuffer<T: Clone> {
    data: Vec<TraitBoundType<T>>,
}

// Test generic buffer with i32 elements
#[test]
fn test_with_generic_buffer() {
    let mut buf: GenericBuffer<i32> = GenericBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
}

// Test generic buffer with String elements
#[test]
fn test_with_generic_strings() {
    let mut buf: GenericBuffer<String> = GenericBuffer::new();
    buf.enqueue("hello".to_string()).unwrap();
    buf.enqueue("world".to_string()).unwrap();
    assert_eq!(buf.dequeue(), Some("hello".to_string()));
    assert_eq!(buf.dequeue(), Some("world".to_string()));
}

// Test buffer with Box smart pointers
#[test]
fn test_with_box() {
    let mut buf = BoxBuffer::new();
    buf.enqueue(Box::new("test".to_string())).unwrap();
    buf.enqueue(Box::new("data".to_string())).unwrap();
    assert_eq!(buf.len(), 2);
    let item = buf.dequeue().unwrap();
    assert_eq!(*item, "test".to_string());
}

// Test buffer with Rc reference-counted pointers
#[test]
fn test_with_rc() {
    let mut buf = RcBuffer::new();
    let item1 = Rc::new("shared1".to_string());
    let item2 = Rc::new("shared2".to_string());
    buf.enqueue(item1.clone()).unwrap();
    buf.enqueue(item2.clone()).unwrap();
    assert_eq!(Rc::strong_count(&item1), 2);
    let dequeued = buf.dequeue().unwrap();
    assert_eq!(*dequeued, "shared1".to_string());
}

// Test buffer with Arc thread-safe pointers
#[test]
fn test_with_arc() {
    let mut buf = ArcBuffer::new();
    let item1 = Arc::new("thread_safe1".to_string());
    let item2 = Arc::new("thread_safe2".to_string());
    buf.enqueue(item1.clone()).unwrap();
    buf.enqueue(item2.clone()).unwrap();
    assert_eq!(buf.len(), 2);
    let dequeued = buf.dequeue().unwrap();
    assert_eq!(*dequeued, "thread_safe1".to_string());
    assert_eq!(buf.len(), 1);
}

// Test buffer with nested generic types (Vec<Vec<T>>)
#[test]
fn test_with_nested_generic() {
    let mut buf: NestedGenericBuffer<i32> = NestedGenericBuffer::new();
    buf.enqueue(vec![1, 2, 3]).unwrap();
    buf.enqueue(vec![4, 5]).unwrap();
    assert_eq!(buf.dequeue(), Some(vec![1, 2, 3]));
    assert_eq!(buf.dequeue(), Some(vec![4, 5]));
}

// Test buffer with trait-bounded custom types
#[test]
fn test_with_trait_bounds() {
    let mut buf: TraitBoundBuffer<i32> = TraitBoundBuffer::new();
    let item1 = TraitBoundType { value: 42 };
    let item2 = TraitBoundType { value: 100 };
    buf.enqueue(item1.clone()).unwrap();
    buf.enqueue(item2.clone()).unwrap();
    assert_eq!(buf.dequeue(), Some(item1));
    assert_eq!(buf.dequeue(), Some(item2));
}

// Test trait-bounded types with String generic parameter
#[test]
fn test_with_trait_bounds_string() {
    let mut buf: TraitBoundBuffer<String> = TraitBoundBuffer::new();
    let item1 = TraitBoundType {
        value: "hello".to_string(),
    };
    let item2 = TraitBoundType {
        value: "world".to_string(),
    };
    buf.enqueue(item1.clone()).unwrap();
    buf.enqueue(item2.clone()).unwrap();
    assert_eq!(buf.dequeue(), Some(item1));
    assert_eq!(buf.dequeue(), Some(item2));
}

// Test Box pointers with complex nested structures
#[test]
fn test_box_with_complex_type() {
    #[derive(Clone, Debug, PartialEq)]
    struct ComplexData {
        id: u64,
        name: String,
        values: Vec<i32>,
    }

    #[ring_buffer(3)]
    struct ComplexBoxBuffer {
        data: Vec<Box<ComplexData>>,
    }

    let mut buf = ComplexBoxBuffer::new();
    let item = Box::new(ComplexData {
        id: 1,
        name: "test".to_string(),
        values: vec![1, 2, 3],
    });
    buf.enqueue(item.clone()).unwrap();
    let dequeued = buf.dequeue().unwrap();
    assert_eq!(dequeued.id, 1);
    assert_eq!(dequeued.name, "test");
}

// Test generic buffer with Option<T> types
#[test]
fn test_generic_with_option() {
    let mut buf: GenericBuffer<Option<i32>> = GenericBuffer::new();
    buf.enqueue(Some(1)).unwrap();
    buf.enqueue(None).unwrap();
    buf.enqueue(Some(2)).unwrap();
    assert_eq!(buf.dequeue(), Some(Some(1)));
    assert_eq!(buf.dequeue(), Some(None));
    assert_eq!(buf.dequeue(), Some(Some(2)));
}

// Test generic buffer with Result<T, E> types
#[test]
fn test_generic_with_result() {
    let mut buf: GenericBuffer<Result<i32, String>> = GenericBuffer::new();
    buf.enqueue(Ok(1)).unwrap();
    buf.enqueue(Err("error".to_string())).unwrap();
    buf.enqueue(Ok(2)).unwrap();
    assert_eq!(buf.dequeue(), Some(Ok(1)));
    assert_eq!(buf.dequeue(), Some(Err("error".to_string())));
    assert_eq!(buf.dequeue(), Some(Ok(2)));
}

// Test Arc with multiple cloned references filling buffer
#[test]
fn test_arc_across_multiple_references() {
    let mut buf = ArcBuffer::new();
    let shared = Arc::new("shared_data".to_string());
    for _ in 0..5 {
        buf.enqueue(shared.clone()).unwrap();
    }
    assert!(buf.is_full());
    let mut count = 0;
    while !buf.is_empty() {
        let item = buf.dequeue().unwrap();
        assert_eq!(*item, "shared_data".to_string());
        count += 1;
    }
    assert_eq!(count, 5);
}

// Test that a newly created buffer is empty with correct initial state
#[test]
fn test_new_buffer_is_empty() {
    let buf = TestBuffer::new();
    assert!(buf.is_empty());
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 0);
    assert_eq!(buf.capacity(), 5);
}

// Test enqueuing a single item updates length and state correctly
#[test]
fn test_enqueue_single_item() {
    let mut buf = TestBuffer::new();
    let result = buf.enqueue(42);
    assert!(result.is_ok());
    assert_eq!(buf.len(), 1);
    assert!(!buf.is_empty());
    assert!(!buf.is_full());
}

// Test enqueuing and then dequeuing a single item returns correct value
#[test]
fn test_enqueue_dequeue_single_item() {
    let mut buf = TestBuffer::new();
    buf.enqueue(42).unwrap();
    let result = buf.dequeue();
    assert_eq!(result, Some(42));
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

// Test enqueuing multiple items updates length correctly
#[test]
fn test_enqueue_multiple_items() {
    let mut buf = TestBuffer::new();
    for i in 1..=3 {
        buf.enqueue(i).unwrap();
    }
    assert_eq!(buf.len(), 3);
    assert!(!buf.is_full());
}

// Test that dequeue maintains FIFO (first-in-first-out) order
#[test]
fn test_dequeue_fifo_order() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
    assert_eq!(buf.dequeue(), Some(3));
}

// Test filling buffer to full capacity sets is_full flag
#[test]
fn test_fill_buffer_to_capacity() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    assert_eq!(buf.len(), 5);
    assert!(buf.is_full());
    assert!(!buf.is_empty());
}

// Test enqueuing to a full buffer returns error with the rejected item
#[test]
fn test_enqueue_when_full_returns_error() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    assert!(buf.is_full());
    let result = buf.enqueue(6);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), 6);
    assert_eq!(buf.len(), 5);
}

// Test dequeuing from an empty buffer returns None
#[test]
fn test_dequeue_from_empty_returns_none() {
    let mut buf = TestBuffer::new();
    let result = buf.dequeue();
    assert_eq!(result, None);
}

// Test that buffer correctly wraps around when head/tail reach the end
#[test]
fn test_wraparound_behavior() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
    assert_eq!(buf.len(), 3);
    buf.enqueue(6).unwrap();
    buf.enqueue(7).unwrap();
    assert_eq!(buf.len(), 5);
    assert!(buf.is_full());
    assert_eq!(buf.dequeue(), Some(3));
    assert_eq!(buf.dequeue(), Some(4));
    assert_eq!(buf.dequeue(), Some(5));
    assert_eq!(buf.dequeue(), Some(6));
    assert_eq!(buf.dequeue(), Some(7));
    assert!(buf.is_empty());
}

// Test that clear resets buffer to empty state
#[test]
fn test_clear() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();
    assert_eq!(buf.len(), 3);
    buf.clear();
    assert_eq!(buf.len(), 0);
    assert!(buf.is_empty());
    buf.enqueue(10).unwrap();
    assert_eq!(buf.dequeue(), Some(10));
}

// Test multiple complete fill/empty cycles to verify wraparound consistency
#[test]
fn test_multiple_wraparounds() {
    let mut buf = TestBuffer::new();
    for cycle in 0..3 {
        for i in 1..=5 {
            buf.enqueue(cycle * 10 + i).unwrap();
        }
        assert!(buf.is_full());
        for i in 1..=5 {
            assert_eq!(buf.dequeue(), Some(cycle * 10 + i));
        }
        assert!(buf.is_empty());
    }
}

// Test alternating enqueue and dequeue operations maintain correct state
#[test]
fn test_alternating_enqueue_dequeue() {
    let mut buf = TestBuffer::new();
    for i in 1..=10 {
        buf.enqueue(i).unwrap();
        assert_eq!(buf.dequeue(), Some(i));
        assert!(buf.is_empty());
    }
}

// Test that len correctly tracks the number of items in the buffer
#[test]
fn test_len_tracking() {
    let mut buf = TestBuffer::new();
    assert_eq!(buf.len(), 0);
    buf.enqueue(1).unwrap();
    assert_eq!(buf.len(), 1);
    buf.enqueue(2).unwrap();
    assert_eq!(buf.len(), 2);
    buf.dequeue();
    assert_eq!(buf.len(), 1);
    buf.dequeue();
    assert_eq!(buf.len(), 0);
}

// Test that capacity remains constant throughout all operations
#[test]
fn test_capacity_unchanged() {
    let mut buf = TestBuffer::new();
    assert_eq!(buf.capacity(), 5);
    buf.enqueue(1).unwrap();
    assert_eq!(buf.capacity(), 5);
    buf.dequeue();
    assert_eq!(buf.capacity(), 5);
    buf.clear();
    assert_eq!(buf.capacity(), 5);
}

// Test complex pattern of interleaved enqueues and dequeues
#[test]
fn test_complex_pattern() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
    buf.enqueue(4).unwrap();
    buf.enqueue(5).unwrap();
    buf.enqueue(6).unwrap();
    assert_eq!(buf.dequeue(), Some(3));
    assert_eq!(buf.dequeue(), Some(4));
    buf.enqueue(7).unwrap();
    buf.enqueue(8).unwrap();
    buf.enqueue(9).unwrap();
    assert!(buf.is_full());
    assert_eq!(buf.dequeue(), Some(5));
    assert_eq!(buf.dequeue(), Some(6));
    assert_eq!(buf.dequeue(), Some(7));
    assert_eq!(buf.dequeue(), Some(8));
    assert_eq!(buf.dequeue(), Some(9));
    assert!(buf.is_empty());
}

// Test clearing a full buffer resets it to empty state
#[test]
fn test_clear_when_full() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    assert!(buf.is_full());
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    buf.enqueue(99).unwrap();
    assert_eq!(buf.dequeue(), Some(99));
}

// Test clearing an already empty buffer has no side effects
#[test]
fn test_clear_when_empty() {
    let mut buf = TestBuffer::new();
    assert!(buf.is_empty());
    buf.clear();
    assert!(buf.is_empty());
    buf.enqueue(1).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
}

// Test multiple consecutive clears work correctly
#[test]
fn test_multiple_clears() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.clear();
    buf.clear();
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
    buf.enqueue(2).unwrap();
    assert_eq!(buf.dequeue(), Some(2));
}

// Test repeatedly dequeuing from empty buffer always returns None
#[test]
fn test_empty_dequeue_repeatedly() {
    let mut buf = TestBuffer::new();
    for _ in 0..10 {
        assert_eq!(buf.dequeue(), None);
    }
    assert!(buf.is_empty());
}

// Test repeatedly enqueuing to full buffer always returns error
#[test]
fn test_full_enqueue_repeatedly() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    for i in 6..=10 {
        let result = buf.enqueue(i);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), i);
    }
    assert_eq!(buf.len(), 5);
    assert!(buf.is_full());
}

// Test enqueuing new items after completely depleting the buffer
#[test]
fn test_enqueue_after_depleting() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    for _ in 0..5 {
        buf.dequeue();
    }
    assert!(buf.is_empty());
    for i in 10..=14 {
        buf.enqueue(i).unwrap();
    }
    assert!(buf.is_full());
    for i in 10..=14 {
        assert_eq!(buf.dequeue(), Some(i));
    }
}

// Test partial fill and drain patterns work correctly
#[test]
fn test_partial_fill_patterns() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
    buf.enqueue(3).unwrap();
    buf.enqueue(4).unwrap();
    buf.enqueue(5).unwrap();
    assert_eq!(buf.dequeue(), Some(3));
    assert_eq!(buf.dequeue(), Some(4));
    assert_eq!(buf.dequeue(), Some(5));
    assert!(buf.is_empty());
}

// Test single slot can be reused many times with immediate dequeue
#[test]
fn test_single_slot_reuse() {
    let mut buf = TestBuffer::new();
    for i in 1..=20 {
        buf.enqueue(i).unwrap();
        assert_eq!(buf.dequeue(), Some(i));
    }
    assert!(buf.is_empty());
}

// Test operations when buffer is near full capacity
#[test]
fn test_near_full_operations() {
    let mut buf = TestBuffer::new();
    for i in 1..=4 {
        buf.enqueue(i).unwrap();
    }
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 4);
    buf.enqueue(5).unwrap();
    assert!(buf.is_full());
    buf.dequeue();
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 4);
}

// Test rapid repeated fill and drain cycles maintain consistency
#[test]
fn test_rapid_operations() {
    let mut buf = TestBuffer::new();
    for _ in 0..1000 {
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        for i in 1..=5 {
            assert_eq!(buf.dequeue(), Some(i));
        }
    }
    assert!(buf.is_empty());
}

// Test interleaved enqueue/dequeue pattern maintains correct order
#[test]
fn test_interleaved_pattern() {
    let mut buf = TestBuffer::new();
    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
    buf.enqueue(3).unwrap();
    buf.enqueue(4).unwrap();
    assert_eq!(buf.dequeue(), Some(2));
    assert_eq!(buf.dequeue(), Some(3));
    buf.enqueue(5).unwrap();
    buf.enqueue(6).unwrap();
    buf.enqueue(7).unwrap();
    assert_eq!(buf.dequeue(), Some(4));
    assert_eq!(buf.dequeue(), Some(5));
    assert_eq!(buf.dequeue(), Some(6));
    assert_eq!(buf.dequeue(), Some(7));
}

// Test buffer state remains unchanged after a failed enqueue
#[test]
fn test_state_after_failed_enqueue() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    let original_len = buf.len();
    let original_full = buf.is_full();
    let result = buf.enqueue(99);
    assert!(result.is_err());
    assert_eq!(buf.len(), original_len);
    assert_eq!(buf.is_full(), original_full);
    assert_eq!(buf.dequeue(), Some(1));
}

// Test clearing buffer during wraparound state resets correctly
#[test]
fn test_clear_during_wraparound() {
    let mut buf = TestBuffer::new();
    for i in 1..=5 {
        buf.enqueue(i).unwrap();
    }
    buf.dequeue();
    buf.dequeue();
    buf.enqueue(6).unwrap();
    buf.enqueue(7).unwrap();
    buf.clear();
    buf.enqueue(100).unwrap();
    assert_eq!(buf.dequeue(), Some(100));
}

// Test state consistency across various operations and transitions
#[test]
fn test_state_consistency() {
    let mut buf = TestBuffer::new();
    assert!(buf.is_empty());
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 0);
    buf.enqueue(1).unwrap();
    assert!(!buf.is_empty());
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 1);
    for i in 2..=5 {
        buf.enqueue(i).unwrap();
    }
    assert!(!buf.is_empty());
    assert!(buf.is_full());
    assert_eq!(buf.len(), 5);
    buf.dequeue();
    assert!(!buf.is_empty());
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 4);
    for _ in 0..4 {
        buf.dequeue();
    }
    assert!(buf.is_empty());
    assert!(!buf.is_full());
    assert_eq!(buf.len(), 0);
}

#[derive(Clone, Debug, PartialEq)]
struct CustomType {
    value: i32,
    name: String,
}

#[ring_buffer(3)]
struct CustomBuffer {
    data: Vec<CustomType>,
}

// Test buffer with custom struct types
#[test]
fn test_with_custom_type() {
    let mut buf = CustomBuffer::new();
    let item1 = CustomType {
        value: 1,
        name: "first".to_string(),
    };
    let item2 = CustomType {
        value: 2,
        name: "second".to_string(),
    };
    buf.enqueue(item1.clone()).unwrap();
    buf.enqueue(item2.clone()).unwrap();
    assert_eq!(buf.dequeue(), Some(item1));
    assert_eq!(buf.dequeue(), Some(item2));
}

#[ring_buffer(10)]
struct StringBuffer {
    data: Vec<String>,
}

// Test buffer with String types
#[test]
fn test_with_strings() {
    let mut buf = StringBuffer::new();
    buf.enqueue("hello".to_string()).unwrap();
    buf.enqueue("world".to_string()).unwrap();
    assert_eq!(buf.dequeue(), Some("hello".to_string()));
    assert_eq!(buf.dequeue(), Some("world".to_string()));
}

#[ring_buffer(1)]
struct TinyBuffer {
    data: Vec<i32>,
}

// Test buffer with capacity of 1 (minimum size)
#[test]
fn test_buffer_size_one() {
    let mut buf = TinyBuffer::new();
    assert_eq!(buf.capacity(), 1);
    buf.enqueue(42).unwrap();
    assert!(buf.is_full());
    assert!(buf.enqueue(43).is_err());
    assert_eq!(buf.dequeue(), Some(42));
    assert!(buf.is_empty());
    buf.enqueue(44).unwrap();
    assert_eq!(buf.dequeue(), Some(44));
}

#[ring_buffer(100)]
struct LargeBuffer {
    data: Vec<i32>,
}

// Test buffer with large capacity (100 elements)
#[test]
fn test_large_buffer() {
    let mut buf = LargeBuffer::new();
    assert_eq!(buf.capacity(), 100);
    for i in 0..100 {
        buf.enqueue(i).unwrap();
    }
    assert!(buf.is_full());
    for i in 0..100 {
        assert_eq!(buf.dequeue(), Some(i));
    }
    assert!(buf.is_empty());
}

#[ring_buffer(5)]
pub struct PublicBuffer {
    data: Vec<i32>,
}

// Test buffer with public visibility
#[test]
fn test_public_buffer() {
    let mut buf = PublicBuffer::new();
    buf.enqueue(1).unwrap();
    assert_eq!(buf.dequeue(), Some(1));
}

#[ring_buffer(5)]
struct OptionBuffer {
    data: Vec<Option<i32>>,
}

// Test buffer containing Option<T> types including None values
#[test]
fn test_with_option_type() {
    let mut buf = OptionBuffer::new();
    buf.enqueue(Some(1)).unwrap();
    buf.enqueue(None).unwrap();
    buf.enqueue(Some(3)).unwrap();
    assert_eq!(buf.dequeue(), Some(Some(1)));
    assert_eq!(buf.dequeue(), Some(None));
    assert_eq!(buf.dequeue(), Some(Some(3)));
}

#[derive(Clone, Debug, PartialEq)]
struct Unit;

#[ring_buffer(5)]
struct UnitBuffer {
    data: Vec<Unit>,
}

// Test buffer with unit-like struct type
#[test]
fn test_with_unit_like_type() {
    let mut buf = UnitBuffer::new();
    buf.enqueue(Unit).unwrap();
    buf.enqueue(Unit).unwrap();
    assert_eq!(buf.len(), 2);
    assert_eq!(buf.dequeue(), Some(Unit));
    assert_eq!(buf.dequeue(), Some(Unit));
}

#[ring_buffer(5)]
struct TupleBuffer {
    data: Vec<(i32, String)>,
}

// Test buffer containing tuple types
#[test]
fn test_with_tuples() {
    let mut buf = TupleBuffer::new();
    buf.enqueue((1, "one".to_string())).unwrap();
    buf.enqueue((2, "two".to_string())).unwrap();
    assert_eq!(buf.dequeue(), Some((1, "one".to_string())));
    assert_eq!(buf.dequeue(), Some((2, "two".to_string())));
}

#[ring_buffer(3)]
struct VecBuffer {
    data: Vec<Vec<i32>>,
}

// Test buffer with nested Vec types including empty vectors
#[test]
fn test_with_nested_vec() {
    let mut buf = VecBuffer::new();
    buf.enqueue(vec![1, 2, 3]).unwrap();
    buf.enqueue(vec![4, 5]).unwrap();
    buf.enqueue(vec![]).unwrap();
    assert_eq!(buf.dequeue(), Some(vec![1, 2, 3]));
    assert_eq!(buf.dequeue(), Some(vec![4, 5]));
    assert_eq!(buf.dequeue(), Some(vec![]));
}

#[ring_buffer(5)]
struct ResultBuffer {
    data: Vec<Result<i32, String>>,
}

// Test buffer containing Result<T, E> types with Ok and Err variants
#[test]
fn test_with_result_type() {
    let mut buf = ResultBuffer::new();
    buf.enqueue(Ok(1)).unwrap();
    buf.enqueue(Err("error".to_string())).unwrap();
    buf.enqueue(Ok(2)).unwrap();
    assert_eq!(buf.dequeue(), Some(Ok(1)));
    assert_eq!(buf.dequeue(), Some(Err("error".to_string())));
    assert_eq!(buf.dequeue(), Some(Ok(2)));
}

#[ring_buffer(1000)]
struct HugeBuffer {
    data: Vec<i32>,
}

// Test buffer with very large capacity (1000 elements) and partial fills
#[test]
fn test_large_capacity() {
    let mut buf = HugeBuffer::new();
    assert_eq!(buf.capacity(), 1000);
    for i in 0..500 {
        buf.enqueue(i).unwrap();
    }
    assert_eq!(buf.len(), 500);
    assert!(!buf.is_full());
    for i in 0..250 {
        assert_eq!(buf.dequeue(), Some(i));
    }
    assert_eq!(buf.len(), 250);
    for i in 500..750 {
        buf.enqueue(i).unwrap();
    }
    assert_eq!(buf.len(), 500);
}
