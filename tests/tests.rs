#![allow(clippy::vec_box, clippy::box_collection)]

use ring_buffer_macro::ring_buffer;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

#[ring_buffer(5)]
struct TestBuffer(i32);

// Basic Operations

mod basic {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf = TestBuffer::new();
        assert!(buf.is_empty());
        assert!(!buf.is_full());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity(), 5);
    }

    #[test]
    fn enqueue_single() {
        let mut buf = TestBuffer::new();
        assert!(buf.enqueue(42).is_ok());
        assert_eq!(buf.len(), 1);
        assert!(!buf.is_empty());
        assert!(!buf.is_full());
    }

    #[test]
    fn enqueue_dequeue_single() {
        let mut buf = TestBuffer::new();
        buf.enqueue(42).unwrap();
        assert_eq!(buf.dequeue(), Some(42));
        assert!(buf.is_empty());
    }

    #[test]
    fn enqueue_multiple() {
        let mut buf = TestBuffer::new();
        for i in 1..=3 {
            buf.enqueue(i).unwrap();
        }
        assert_eq!(buf.len(), 3);
        assert!(!buf.is_full());
    }

    #[test]
    fn fifo_order() {
        let mut buf = TestBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        assert_eq!(buf.dequeue(), Some(1));
        assert_eq!(buf.dequeue(), Some(2));
        assert_eq!(buf.dequeue(), Some(3));
    }

    #[test]
    fn fill_to_capacity() {
        let mut buf = TestBuffer::new();
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        assert_eq!(buf.len(), 5);
        assert!(buf.is_full());
        assert!(!buf.is_empty());
    }

    #[test]
    fn enqueue_full_returns_error() {
        let mut buf = TestBuffer::new();
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        let result = buf.enqueue(6);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 6);
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn dequeue_empty_returns_none() {
        let mut buf = TestBuffer::new();
        assert_eq!(buf.dequeue(), None);
    }

    #[test]
    fn len_tracking() {
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

    #[test]
    fn capacity_unchanged() {
        let mut buf = TestBuffer::new();
        assert_eq!(buf.capacity(), 5);
        buf.enqueue(1).unwrap();
        assert_eq!(buf.capacity(), 5);
        buf.dequeue();
        assert_eq!(buf.capacity(), 5);
        buf.clear();
        assert_eq!(buf.capacity(), 5);
    }

    #[test]
    fn state_consistency() {
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

    #[test]
    fn state_after_failed_enqueue() {
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
}

// Wraparound & Cycling

mod wraparound {
    use super::*;

    #[test]
    fn basic_wraparound() {
        let mut buf = TestBuffer::new();
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        assert_eq!(buf.dequeue(), Some(1));
        assert_eq!(buf.dequeue(), Some(2));
        buf.enqueue(6).unwrap();
        buf.enqueue(7).unwrap();
        assert!(buf.is_full());
        assert_eq!(buf.dequeue(), Some(3));
        assert_eq!(buf.dequeue(), Some(4));
        assert_eq!(buf.dequeue(), Some(5));
        assert_eq!(buf.dequeue(), Some(6));
        assert_eq!(buf.dequeue(), Some(7));
        assert!(buf.is_empty());
    }

    #[test]
    fn multiple_cycles() {
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

    #[test]
    fn alternating() {
        let mut buf = TestBuffer::new();
        for i in 1..=10 {
            buf.enqueue(i).unwrap();
            assert_eq!(buf.dequeue(), Some(i));
            assert!(buf.is_empty());
        }
    }

    #[test]
    fn single_slot_reuse() {
        let mut buf = TestBuffer::new();
        for i in 1..=20 {
            buf.enqueue(i).unwrap();
            assert_eq!(buf.dequeue(), Some(i));
        }
        assert!(buf.is_empty());
    }

    #[test]
    fn rapid_fill_drain() {
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

    #[test]
    fn interleaved() {
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

    #[test]
    fn complex_pattern() {
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
}

// Clear

mod clear {
    use super::*;

    #[test]
    fn basic() {
        let mut buf = TestBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        buf.enqueue(10).unwrap();
        assert_eq!(buf.dequeue(), Some(10));
    }

    #[test]
    fn when_full() {
        let mut buf = TestBuffer::new();
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        buf.clear();
        assert!(buf.is_empty());
        buf.enqueue(99).unwrap();
        assert_eq!(buf.dequeue(), Some(99));
    }

    #[test]
    fn when_empty() {
        let mut buf = TestBuffer::new();
        buf.clear();
        assert!(buf.is_empty());
        buf.enqueue(1).unwrap();
        assert_eq!(buf.dequeue(), Some(1));
    }

    #[test]
    fn multiple() {
        let mut buf = TestBuffer::new();
        buf.enqueue(1).unwrap();
        buf.clear();
        buf.clear();
        buf.clear();
        assert!(buf.is_empty());
        buf.enqueue(2).unwrap();
        assert_eq!(buf.dequeue(), Some(2));
    }

    #[test]
    fn during_wraparound() {
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
}

// Edge Cases

mod edge_cases {
    use super::*;

    #[test]
    fn dequeue_empty_repeatedly() {
        let mut buf = TestBuffer::new();
        for _ in 0..10 {
            assert_eq!(buf.dequeue(), None);
        }
        assert!(buf.is_empty());
    }

    #[test]
    fn enqueue_full_repeatedly() {
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
    }

    #[test]
    fn enqueue_after_depleting() {
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

    #[test]
    fn partial_fill() {
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

    #[test]
    fn near_full() {
        let mut buf = TestBuffer::new();
        for i in 1..=4 {
            buf.enqueue(i).unwrap();
        }
        assert!(!buf.is_full());
        buf.enqueue(5).unwrap();
        assert!(buf.is_full());
        buf.dequeue();
        assert!(!buf.is_full());
        assert_eq!(buf.len(), 4);
    }

    #[ring_buffer(1)]
    struct TinyBuffer(i32);

    #[test]
    fn buffer_size_one() {
        let mut buf = TinyBuffer::new();
        assert_eq!(buf.capacity(), 1);
        buf.enqueue(42).unwrap();
        assert!(buf.is_full());
        assert!(buf.enqueue(43).is_err());
        assert_eq!(buf.dequeue(), Some(42));
        assert!(buf.is_empty());
    }

    #[ring_buffer(1000)]
    struct HugeBuffer(i32);

    #[test]
    fn large_capacity() {
        let mut buf = HugeBuffer::new();
        assert_eq!(buf.capacity(), 1000);
        for i in 0..500 {
            buf.enqueue(i).unwrap();
        }
        assert_eq!(buf.len(), 500);
        for i in 0..250 {
            assert_eq!(buf.dequeue(), Some(i));
        }
        assert_eq!(buf.len(), 250);
        for i in 500..750 {
            buf.enqueue(i).unwrap();
        }
        assert_eq!(buf.len(), 500);
    }
}

// Various Element Types

mod types {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct CustomType {
        value: i32,
        name: String,
    }

    #[ring_buffer(3)]
    struct CustomBuffer(CustomType);

    #[test]
    fn custom_struct() {
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
    struct StringBuffer(String);

    #[test]
    fn strings() {
        let mut buf = StringBuffer::new();
        buf.enqueue("hello".to_string()).unwrap();
        buf.enqueue("world".to_string()).unwrap();
        assert_eq!(buf.dequeue(), Some("hello".to_string()));
        assert_eq!(buf.dequeue(), Some("world".to_string()));
    }

    #[ring_buffer(5)]
    struct OptionBuffer(Option<i32>);

    #[test]
    fn option_type() {
        let mut buf = OptionBuffer::new();
        buf.enqueue(Some(1)).unwrap();
        buf.enqueue(None).unwrap();
        buf.enqueue(Some(3)).unwrap();
        assert_eq!(buf.dequeue(), Some(Some(1)));
        assert_eq!(buf.dequeue(), Some(None));
        assert_eq!(buf.dequeue(), Some(Some(3)));
    }

    #[ring_buffer(5)]
    struct ResultBuffer(Result<i32, String>);

    #[test]
    fn result_type() {
        let mut buf = ResultBuffer::new();
        buf.enqueue(Ok(1)).unwrap();
        buf.enqueue(Err("error".to_string())).unwrap();
        buf.enqueue(Ok(2)).unwrap();
        assert_eq!(buf.dequeue(), Some(Ok(1)));
        assert_eq!(buf.dequeue(), Some(Err("error".to_string())));
        assert_eq!(buf.dequeue(), Some(Ok(2)));
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Unit;

    #[ring_buffer(5)]
    struct UnitBuffer(Unit);

    #[test]
    fn unit_like() {
        let mut buf = UnitBuffer::new();
        buf.enqueue(Unit).unwrap();
        buf.enqueue(Unit).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.dequeue(), Some(Unit));
    }

    #[ring_buffer(5)]
    struct TupleBuffer((i32, String));

    #[test]
    fn tuples() {
        let mut buf = TupleBuffer::new();
        buf.enqueue((1, "one".to_string())).unwrap();
        buf.enqueue((2, "two".to_string())).unwrap();
        assert_eq!(buf.dequeue(), Some((1, "one".to_string())));
        assert_eq!(buf.dequeue(), Some((2, "two".to_string())));
    }

    #[ring_buffer(3)]
    struct VecBuffer(Vec<i32>);

    #[test]
    fn nested_vec() {
        let mut buf = VecBuffer::new();
        buf.enqueue(vec![1, 2, 3]).unwrap();
        buf.enqueue(vec![4, 5]).unwrap();
        buf.enqueue(vec![]).unwrap();
        assert_eq!(buf.dequeue(), Some(vec![1, 2, 3]));
        assert_eq!(buf.dequeue(), Some(vec![4, 5]));
        assert_eq!(buf.dequeue(), Some(vec![]));
    }

    #[ring_buffer(5)]
    pub struct PublicBuffer(i32);

    #[test]
    fn public_visibility() {
        let mut buf = PublicBuffer::new();
        buf.enqueue(1).unwrap();
        assert_eq!(buf.dequeue(), Some(1));
    }

    #[ring_buffer(100)]
    struct LargeBuffer(i32);

    #[test]
    fn large_buffer() {
        let mut buf = LargeBuffer::new();
        assert_eq!(buf.capacity(), 100);
        for i in 0..100 {
            buf.enqueue(i).unwrap();
        }
        assert!(buf.is_full());
        for i in 0..100 {
            assert_eq!(buf.dequeue(), Some(i));
        }
    }
}

// Generics & Trait Bounds

mod generics {
    use super::*;

    #[ring_buffer(10)]
    struct GenericBuffer<T: Clone>(T);

    #[test]
    fn with_i32() {
        let mut buf: GenericBuffer<i32> = GenericBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        assert_eq!(buf.dequeue(), Some(1));
        assert_eq!(buf.dequeue(), Some(2));
    }

    #[test]
    fn with_string() {
        let mut buf: GenericBuffer<String> = GenericBuffer::new();
        buf.enqueue("hello".to_string()).unwrap();
        buf.enqueue("world".to_string()).unwrap();
        assert_eq!(buf.dequeue(), Some("hello".to_string()));
        assert_eq!(buf.dequeue(), Some("world".to_string()));
    }

    #[test]
    fn with_option() {
        let mut buf: GenericBuffer<Option<i32>> = GenericBuffer::new();
        buf.enqueue(Some(1)).unwrap();
        buf.enqueue(None).unwrap();
        buf.enqueue(Some(2)).unwrap();
        assert_eq!(buf.dequeue(), Some(Some(1)));
        assert_eq!(buf.dequeue(), Some(None));
        assert_eq!(buf.dequeue(), Some(Some(2)));
    }

    #[test]
    fn with_result() {
        let mut buf: GenericBuffer<Result<i32, String>> = GenericBuffer::new();
        buf.enqueue(Ok(1)).unwrap();
        buf.enqueue(Err("error".to_string())).unwrap();
        assert_eq!(buf.dequeue(), Some(Ok(1)));
        assert_eq!(buf.dequeue(), Some(Err("error".to_string())));
    }

    #[ring_buffer(5)]
    struct NestedGenericBuffer<T: Clone>(Vec<T>);

    #[test]
    fn nested_generic() {
        let mut buf: NestedGenericBuffer<i32> = NestedGenericBuffer::new();
        buf.enqueue(vec![1, 2, 3]).unwrap();
        buf.enqueue(vec![4, 5]).unwrap();
        assert_eq!(buf.dequeue(), Some(vec![1, 2, 3]));
        assert_eq!(buf.dequeue(), Some(vec![4, 5]));
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TraitBoundType<T: Clone> {
        value: T,
    }

    #[ring_buffer(5)]
    struct TraitBoundBuffer<T: Clone>(TraitBoundType<T>);

    #[test]
    fn trait_bounds_i32() {
        let mut buf: TraitBoundBuffer<i32> = TraitBoundBuffer::new();
        let item1 = TraitBoundType { value: 42 };
        let item2 = TraitBoundType { value: 100 };
        buf.enqueue(item1.clone()).unwrap();
        buf.enqueue(item2.clone()).unwrap();
        assert_eq!(buf.dequeue(), Some(item1));
        assert_eq!(buf.dequeue(), Some(item2));
    }

    #[test]
    fn trait_bounds_string() {
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

    #[ring_buffer(5)]
    struct GenericIterBuffer<T: Clone>(T);

    #[test]
    fn generic_iter() {
        let mut buf: GenericIterBuffer<i32> = GenericIterBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        let sum: i32 = buf.iter().sum();
        assert_eq!(sum, 6);
    }
}

// Smart Pointers

mod smart_pointers {
    use super::*;

    #[ring_buffer(5)]
    struct BoxBuffer(Box<String>);

    #[test]
    fn boxed() {
        let mut buf = BoxBuffer::new();
        buf.enqueue(Box::new("test".to_string())).unwrap();
        buf.enqueue(Box::new("data".to_string())).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(*buf.dequeue().unwrap(), "test".to_string());
    }

    #[test]
    fn box_complex() {
        #[derive(Clone, Debug, PartialEq)]
        struct ComplexData {
            id: u64,
            name: String,
            values: Vec<i32>,
        }

        #[ring_buffer(3)]
        struct ComplexBoxBuffer(Box<ComplexData>);

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

    #[ring_buffer(5)]
    struct RcBuffer(Rc<String>);

    #[test]
    fn rc() {
        let mut buf = RcBuffer::new();
        let item1 = Rc::new("shared1".to_string());
        let item2 = Rc::new("shared2".to_string());
        buf.enqueue(item1.clone()).unwrap();
        buf.enqueue(item2.clone()).unwrap();
        assert_eq!(Rc::strong_count(&item1), 2);
        assert_eq!(*buf.dequeue().unwrap(), "shared1".to_string());
    }

    #[ring_buffer(5)]
    struct ArcBuffer(Arc<String>);

    #[test]
    fn arc() {
        let mut buf = ArcBuffer::new();
        let item1 = Arc::new("thread_safe1".to_string());
        let item2 = Arc::new("thread_safe2".to_string());
        buf.enqueue(item1.clone()).unwrap();
        buf.enqueue(item2.clone()).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(*buf.dequeue().unwrap(), "thread_safe1".to_string());
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn arc_multiple_refs() {
        let mut buf = ArcBuffer::new();
        let shared = Arc::new("shared_data".to_string());
        for _ in 0..5 {
            buf.enqueue(shared.clone()).unwrap();
        }
        assert!(buf.is_full());
        let mut count = 0;
        while !buf.is_empty() {
            assert_eq!(*buf.dequeue().unwrap(), "shared_data".to_string());
            count += 1;
        }
        assert_eq!(count, 5);
    }
}

// Peek, Iter, Drain

mod peek_and_iter {
    use super::*;

    #[ring_buffer(5)]
    struct PeekBuffer(i32);

    #[test]
    fn peek_empty() {
        let buf = PeekBuffer::new();
        assert_eq!(buf.peek(), None);
    }

    #[test]
    fn peek_with_items() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(42).unwrap();
        buf.enqueue(100).unwrap();
        assert_eq!(buf.peek(), Some(&42));
        assert_eq!(buf.len(), 2);
        buf.dequeue();
        assert_eq!(buf.peek(), Some(&100));
    }

    #[test]
    fn peek_mut() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(10).unwrap();
        if let Some(val) = buf.peek_mut() {
            *val = 99;
        }
        assert_eq!(buf.dequeue(), Some(99));
    }

    #[test]
    fn peek_back() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        assert_eq!(buf.peek(), Some(&1));
        assert_eq!(buf.peek_back(), Some(&3));
    }

    #[test]
    fn iter() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        let collected: Vec<_> = buf.iter().cloned().collect();
        assert_eq!(collected, vec![1, 2, 3]);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn iter_wraparound() {
        let mut buf = PeekBuffer::new();
        for i in 1..=5 {
            buf.enqueue(i).unwrap();
        }
        buf.dequeue();
        buf.dequeue();
        buf.enqueue(6).unwrap();
        buf.enqueue(7).unwrap();
        let collected: Vec<_> = buf.iter().cloned().collect();
        assert_eq!(collected, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn drain() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        let collected: Vec<_> = buf.drain().collect();
        assert_eq!(collected, vec![1, 2, 3]);
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_partial() {
        let mut buf = PeekBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        buf.enqueue(3).unwrap();
        let mut drain = buf.drain();
        assert_eq!(drain.next(), Some(1));
        assert_eq!(drain.next(), Some(2));
        drop(drain);
        assert!(buf.is_empty());
    }
}

// Buffer Options

mod buffer_options {
    use super::*;

    #[ring_buffer(capacity = 8, power_of_two = true)]
    struct PowerOfTwoBuffer(i32);

    #[test]
    fn power_of_two_basic() {
        let mut buf = PowerOfTwoBuffer::new();
        assert_eq!(buf.capacity(), 8);
        for i in 0..8 {
            buf.enqueue(i).unwrap();
        }
        assert!(buf.is_full());
        for i in 0..8 {
            assert_eq!(buf.dequeue(), Some(i));
        }
        assert!(buf.is_empty());
    }

    #[test]
    fn power_of_two_wraparound() {
        let mut buf = PowerOfTwoBuffer::new();
        for cycle in 0..5 {
            for i in 0..8 {
                buf.enqueue(cycle * 10 + i).unwrap();
            }
            for i in 0..8 {
                assert_eq!(buf.dequeue(), Some(cycle * 10 + i));
            }
        }
    }

    #[ring_buffer(capacity = 16, cache_padded = true)]
    struct CachePaddedBuffer(i32);

    #[test]
    fn cache_padded() {
        let mut buf = CachePaddedBuffer::new();
        buf.enqueue(1).unwrap();
        buf.enqueue(2).unwrap();
        assert_eq!(buf.dequeue(), Some(1));
        assert_eq!(buf.dequeue(), Some(2));
    }

    #[ring_buffer(capacity = 32, power_of_two = true, cache_padded = true)]
    struct OptimizedBuffer(i32);

    #[test]
    fn combined_options() {
        let mut buf = OptimizedBuffer::new();
        assert_eq!(buf.capacity(), 32);
        for i in 0..32 {
            buf.enqueue(i).unwrap();
        }
        assert!(buf.is_full());
        for i in 0..32 {
            assert_eq!(buf.dequeue(), Some(i));
        }
    }

    #[ring_buffer(capacity = 10, mode = "standard")]
    struct NamedParamsBuffer(String);

    #[test]
    fn named_params() {
        let mut buf = NamedParamsBuffer::new();
        buf.enqueue("hello".to_string()).unwrap();
        buf.enqueue("world".to_string()).unwrap();
        assert_eq!(buf.peek(), Some(&"hello".to_string()));
        assert_eq!(buf.dequeue(), Some("hello".to_string()));
    }
}

// SPSC

mod spsc {
    use super::*;

    #[ring_buffer(capacity = 5, mode = "spsc")]
    struct SpscBuffer(i32);

    #[ring_buffer(capacity = 10, mode = "spsc")]
    struct SpscStringBuffer(String);

    #[ring_buffer(capacity = 1024, mode = "spsc")]
    struct LargeSpscBuffer(u64);

    #[ring_buffer(capacity = 2, mode = "spsc")]
    struct TinySpscBuffer(i32);

    #[ring_buffer(capacity = 8, mode = "spsc")]
    struct SpscPeekBuffer(i32);

    #[ring_buffer(capacity = 16, mode = "spsc", power_of_two = true)]
    struct OptimizedSpscBuffer(i32);

    #[ring_buffer(capacity = 16, mode = "spsc", cache_padded = true)]
    struct CachePaddedSpscBuffer(i32);

    #[test]
    fn new_buffer() {
        let buf = SpscBuffer::new();
        assert!(buf.is_empty());
        assert!(!buf.is_full());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity(), 5);
    }

    #[test]
    fn basic_operations() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        assert!(producer.try_enqueue(1).is_ok());
        assert!(producer.try_enqueue(2).is_ok());
        assert_eq!(producer.len(), 2);
        assert_eq!(consumer.try_dequeue(), Some(1));
        assert_eq!(consumer.try_dequeue(), Some(2));
        assert!(consumer.is_empty());
    }

    #[test]
    fn fifo_order() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        for i in 1..=4 {
            producer.try_enqueue(i).unwrap();
        }
        for i in 1..=4 {
            assert_eq!(consumer.try_dequeue(), Some(i));
        }
    }

    #[test]
    fn full_buffer() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        for i in 1..=4 {
            assert!(producer.try_enqueue(i).is_ok());
        }
        assert!(producer.is_full());
        let result = producer.try_enqueue(100);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 100);
        assert_eq!(consumer.try_dequeue(), Some(1));
        assert!(!producer.is_full());
        assert!(producer.try_enqueue(5).is_ok());
    }

    #[test]
    fn empty_dequeue() {
        let buf = SpscBuffer::new();
        let (_, consumer) = buf.split();
        assert_eq!(consumer.try_dequeue(), None);
        assert_eq!(consumer.try_dequeue(), None);
    }

    #[test]
    fn wraparound() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        for cycle in 0..10 {
            for i in 0..4 {
                producer.try_enqueue(cycle * 10 + i).unwrap();
            }
            for i in 0..4 {
                assert_eq!(consumer.try_dequeue(), Some(cycle * 10 + i));
            }
        }
    }

    #[test]
    fn with_strings() {
        let buf = SpscStringBuffer::new();
        let (producer, consumer) = buf.split();
        producer.try_enqueue("hello".to_string()).unwrap();
        producer.try_enqueue("world".to_string()).unwrap();
        assert_eq!(consumer.try_dequeue(), Some("hello".to_string()));
        assert_eq!(consumer.try_dequeue(), Some("world".to_string()));
    }

    #[test]
    fn alternating() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        for i in 0..100 {
            producer.try_enqueue(i).unwrap();
            assert_eq!(consumer.try_dequeue(), Some(i));
        }
    }

    #[test]
    fn len_tracking() {
        let buf = SpscBuffer::new();
        let (producer, consumer) = buf.split();
        assert_eq!(producer.len(), 0);
        producer.try_enqueue(1).unwrap();
        assert_eq!(producer.len(), 1);
        producer.try_enqueue(2).unwrap();
        assert_eq!(consumer.len(), 2);
        consumer.try_dequeue();
        assert_eq!(producer.len(), 1);
        consumer.try_dequeue();
        assert_eq!(consumer.len(), 0);
    }

    #[test]
    fn tiny_buffer() {
        let buf = TinySpscBuffer::new();
        let (producer, consumer) = buf.split();
        producer.try_enqueue(42).unwrap();
        assert!(producer.is_full());
        assert!(producer.try_enqueue(43).is_err());
        assert_eq!(consumer.try_dequeue(), Some(42));
        assert!(consumer.is_empty());
        producer.try_enqueue(44).unwrap();
        assert_eq!(consumer.try_dequeue(), Some(44));
    }

    #[test]
    fn peek() {
        let buf = SpscPeekBuffer::new();
        let (producer, consumer) = buf.split();
        producer.try_enqueue(42).unwrap();
        producer.try_enqueue(100).unwrap();
        assert_eq!(consumer.peek(), Some(&42));
        assert_eq!(consumer.peek(), Some(&42));
        assert_eq!(consumer.try_dequeue(), Some(42));
        assert_eq!(consumer.peek(), Some(&100));
    }

    #[test]
    fn power_of_two() {
        let buf = OptimizedSpscBuffer::new();
        let (producer, consumer) = buf.split();
        for i in 0..15 {
            producer.try_enqueue(i).unwrap();
        }
        for i in 0..15 {
            assert_eq!(consumer.try_dequeue(), Some(i));
        }
    }

    #[test]
    fn cache_padded() {
        let buf = CachePaddedSpscBuffer::new();
        let (producer, consumer) = buf.split();
        for i in 0..15 {
            producer.try_enqueue(i).unwrap();
        }
        for i in 0..15 {
            assert_eq!(consumer.try_dequeue(), Some(i));
        }
    }

    #[test]
    fn threaded_basic() {
        let buf = Arc::new(SpscBuffer::new());
        let buf_producer = Arc::clone(&buf);
        let buf_consumer = Arc::clone(&buf);

        let producer_handle = thread::spawn(move || {
            let (producer, _) = buf_producer.split();
            for i in 0..4 {
                while producer.try_enqueue(i).is_err() {
                    thread::yield_now();
                }
            }
        });

        let consumer_handle = thread::spawn(move || {
            let (_, consumer) = buf_consumer.split();
            let mut received = Vec::new();
            while received.len() < 4 {
                if let Some(item) = consumer.try_dequeue() {
                    received.push(item);
                } else {
                    thread::yield_now();
                }
            }
            received
        });

        producer_handle.join().unwrap();
        let received = consumer_handle.join().unwrap();
        assert_eq!(received, vec![0, 1, 2, 3]);
    }

    #[test]
    fn high_throughput() {
        let buf = Arc::new(LargeSpscBuffer::new());
        let count = 100_000u64;
        let buf_producer = Arc::clone(&buf);
        let buf_consumer = Arc::clone(&buf);

        let producer_handle = thread::spawn(move || {
            let (producer, _) = buf_producer.split();
            for i in 0..count {
                while producer.try_enqueue(i).is_err() {}
            }
        });

        let consumer_handle = thread::spawn(move || {
            let (_, consumer) = buf_consumer.split();
            let mut sum = 0u64;
            let mut received = 0u64;
            while received < count {
                if let Some(item) = consumer.try_dequeue() {
                    sum += item;
                    received += 1;
                }
            }
            sum
        });

        producer_handle.join().unwrap();
        let sum = consumer_handle.join().unwrap();
        assert_eq!(sum, (count - 1) * count / 2);
    }
}

// MPSC

mod mpsc {
    use super::*;

    #[ring_buffer(capacity = 64, mode = "mpsc")]
    struct MpscBuffer(i32);

    #[ring_buffer(capacity = 16, mode = "mpsc")]
    struct MpscStringBuffer(String);

    #[ring_buffer(capacity = 64, mode = "mpsc", power_of_two = true)]
    struct MpscPow2Buffer(i32);

    #[test]
    fn basic() {
        let buffer = MpscBuffer::new();
        let producer = buffer.producer();
        let consumer = buffer.consumer();
        producer.try_enqueue(1).unwrap();
        producer.try_enqueue(2).unwrap();
        producer.try_enqueue(3).unwrap();
        assert_eq!(consumer.try_dequeue(), Some(1));
        assert_eq!(consumer.try_dequeue(), Some(2));
        assert_eq!(consumer.try_dequeue(), Some(3));
        assert_eq!(consumer.try_dequeue(), None);
    }

    #[test]
    fn multiple_producers() {
        let buffer = Arc::new(MpscBuffer::new());
        let mut handles = vec![];
        for i in 0..4 {
            let buf = Arc::clone(&buffer);
            handles.push(thread::spawn(move || {
                let producer = buf.producer();
                for j in 0..10 {
                    while producer.try_enqueue(i * 10 + j).is_err() {
                        std::hint::spin_loop();
                    }
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let consumer = buffer.consumer();
        let mut count = 0;
        while consumer.try_dequeue().is_some() {
            count += 1;
        }
        assert_eq!(count, 40);
    }

    #[test]
    fn producer_consumer_concurrent() {
        let buffer = Arc::new(MpscBuffer::new());
        let buf1 = Arc::clone(&buffer);
        let buf2 = Arc::clone(&buffer);

        let producer = thread::spawn(move || {
            let p = buf1.producer();
            for i in 0..100 {
                while p.try_enqueue(i).is_err() {
                    std::hint::spin_loop();
                }
            }
        });

        let consumer = thread::spawn(move || {
            let c = buf2.consumer();
            let mut received = 0;
            while received < 100 {
                if c.try_dequeue().is_some() {
                    received += 1;
                } else {
                    std::hint::spin_loop();
                }
            }
            received
        });

        producer.join().unwrap();
        assert_eq!(consumer.join().unwrap(), 100);
    }

    #[test]
    fn peek() {
        let buffer = MpscBuffer::new();
        let producer = buffer.producer();
        let consumer = buffer.consumer();
        assert!(consumer.peek().is_none());
        producer.try_enqueue(42).unwrap();
        assert_eq!(consumer.peek(), Some(&42));
        assert_eq!(consumer.peek(), Some(&42));
        assert_eq!(consumer.try_dequeue(), Some(42));
        assert!(consumer.peek().is_none());
    }

    #[test]
    fn with_strings() {
        let buffer = MpscStringBuffer::new();
        let producer = buffer.producer();
        let consumer = buffer.consumer();
        producer.try_enqueue("hello".to_string()).unwrap();
        producer.try_enqueue("world".to_string()).unwrap();
        assert_eq!(consumer.try_dequeue(), Some("hello".to_string()));
        assert_eq!(consumer.try_dequeue(), Some("world".to_string()));
    }

    #[test]
    fn power_of_two() {
        let buffer = MpscPow2Buffer::new();
        let producer = buffer.producer();
        let consumer = buffer.consumer();
        for i in 0..63 {
            producer.try_enqueue(i).unwrap();
        }
        assert!(producer.is_full());
        for i in 0..63 {
            assert_eq!(consumer.try_dequeue(), Some(i));
        }
        assert!(consumer.is_empty());
    }
}

// Blocking Variants

mod blocking {
    use super::*;

    #[ring_buffer(capacity = 8, mode = "spsc", blocking = true)]
    struct BlockingSpscBuffer(i32);

    #[test]
    fn spsc() {
        let buffer = Arc::new(BlockingSpscBuffer::new());
        let buf1 = Arc::clone(&buffer);
        let buf2 = Arc::clone(&buffer);

        let producer = thread::spawn(move || {
            let (p, _) = buf1.split();
            for i in 0..20 {
                p.enqueue_blocking(i);
            }
        });

        let consumer = thread::spawn(move || {
            let (_, c) = buf2.split();
            let mut sum = 0;
            for _ in 0..20 {
                sum += c.dequeue_blocking();
            }
            sum
        });

        producer.join().unwrap();
        assert_eq!(consumer.join().unwrap(), (0..20).sum::<i32>());
    }

    #[ring_buffer(capacity = 8, mode = "mpsc", blocking = true)]
    struct BlockingMpscBuffer(i32);

    #[test]
    fn mpsc() {
        let buffer = Arc::new(BlockingMpscBuffer::new());
        let mut handles = vec![];

        for i in 0..2 {
            let buf = Arc::clone(&buffer);
            handles.push(thread::spawn(move || {
                let p = buf.producer();
                for j in 0..10 {
                    p.enqueue_blocking(i * 10 + j);
                }
            }));
        }

        let buf = Arc::clone(&buffer);
        let consumer = thread::spawn(move || {
            let c = buf.consumer();
            let mut sum = 0;
            for _ in 0..20 {
                sum += c.dequeue_blocking();
            }
            sum
        });

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(consumer.join().unwrap(), (0..20).sum::<i32>());
    }
}
