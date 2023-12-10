use alloc::boxed::Box;

use core::{
    cmp::Ordering,
    ops::Index,
    ptr
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum RBColor {
    Red,
    Black
}

/* RBNode */
struct RBNode<K: Ord, V> {
    color: RBColor,
    left: NodePtr<K, V>,
    right: NodePtr<K, V>,
    parent: NodePtr<K, V>,
    key: K,
    value: V
}

impl<K: Ord, V> RBNode<K, V> {
    #[inline(always)]
    fn pair(self) -> (K, V) {
        (self.key, self.value)
    }
}

/* NodePtr */
struct NodePtr<K: Ord, V>(*mut RBNode<K, V>);

impl<K: Ord, V> Clone for NodePtr<K, V> {
    fn clone(&self) -> Self {
        NodePtr(self.0)
    }
}

impl<K: Ord, V> Copy for NodePtr<K, V> {}

impl<K: Ord, V> Ord for NodePtr<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { (*self.0).key.cmp(&(*other.0).key) }
    }
}

impl<K: Ord, V> PartialOrd for NodePtr<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { Some((*self.0).key.cmp(&(*other.0).key)) }
    }
}

impl<K: Ord, V> PartialEq for NodePtr<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Ord, V> Eq for NodePtr<K, V> {}

impl<K: Ord, V> NodePtr<K, V> {
    fn new(k: K, v: V) -> Self {
        let node = RBNode {
            color: RBColor::Black,
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            key: k,
            value: v
        };
        return Self(Box::into_raw(Box::new(node)))
    }

    #[inline(always)]
    fn set_color(&mut self, color: RBColor) {
        if self.is_null() {
            return
        }
        unsafe {
            (*self.0).color = color
        }
    }

    #[inline(always)]
    fn get_color(&self) -> RBColor {
        if self.is_null() {
            return RBColor::Black
        }
        unsafe { (*self.0).color }
    }

    #[inline(always)]
    fn is_black_color(&self) -> bool {
        if self.is_null() {
            return true
        }
        unsafe { (*self.0).color == RBColor::Black }
    }
    
    #[inline(always)]
    fn is_red_color(&self) -> bool {
        if self.is_null() {
            return false
        }
        unsafe { (*self.0).color == RBColor::Red }
    }

    #[inline(always)]
    fn min_node(self) -> NodePtr<K, V> {
        let mut temp = self.clone();
        while !temp.left().is_null() {
            temp = temp.left();
        }
        return temp
    }

    #[inline(always)]
    fn set_parent(&mut self, parent: Self) {
        if self.is_null() {
            return
        }
        unsafe { (*self.0).parent = parent }
    }

    #[inline(always)]
    fn set_left(&mut self, left: Self) {
        if self.is_null() {
            return
        }
        unsafe { (*self.0).left = left }
    }

    #[inline(always)]
    fn set_right(&mut self, right: Self) {
        if self.is_null() {
            return
        }
        unsafe { (*self.0).right = right }
    }

    #[inline(always)]
    fn parent(&self) -> Self {
        if self.is_null() {
            return NodePtr::null()
        }
        unsafe { (*self.0).parent.clone() }
    }

    #[inline(always)]
    fn left(&self) -> Self {
        if self.is_null() {
            return NodePtr::null()
        }
        unsafe { (*self.0).left.clone() }
    }

    #[inline(always)]
    fn right(&self) -> Self {
        if self.is_null() {
            return NodePtr::null()
        }
        unsafe { (*self.0).right.clone() }
    }

    #[inline(always)]
    fn null() -> Self {
        Self(ptr::null_mut())
    }

    #[inline(always)]
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

/* RBTree */
pub struct RBTree<K: Ord, V> {
    root: NodePtr<K, V>,
    len: usize
}

impl<'a, K, V> Index<&'a K> for RBTree<K, V> 
where
    K: Ord 
{
    type Output = V;
    #[inline(always)]
    fn index(&self, index: &K) -> &Self::Output {
        self.get(index).expect("no entry found for the given key")
    }
}

impl<K: Ord, V> RBTree<K, V> {
    pub fn new() -> Self {
        Self {
            root: NodePtr::null(),
            len: 0
        }
    }

    #[inline(always)]
    unsafe fn left_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.right();
        node.set_right(temp.left());

        if !temp.left().is_null() {
            temp.left().set_parent(node.clone());
        }

        temp.set_parent(node.parent());
        if node == self.root {
            self.root = temp.clone();
        } else if node == node.parent().left() {
            node.parent().set_left(temp.clone());
        } else {
            node.parent().set_right(temp.clone());
        }

        temp.set_left(node.clone());
        node.set_parent(temp.clone());
    }

    #[inline(always)]
    fn right_rotate(&mut self, mut node: NodePtr<K, V>) {
        let mut temp = node.left();
        node.set_left(temp.right());

        if !temp.right().is_null() {
            temp.right().set_parent(node.clone());
        }

        temp.set_parent(node.parent());
        if node == self.root {
            self.root = temp.clone();
        } else if node == node.parent().right() {
            node.parent().set_right(temp.clone());
        } else {
            node.parent().set_left(temp.clone());
        }

        temp.set_right(node.clone());
        node.set_parent(temp.clone());
    }
    
    #[inline(always)]
    unsafe fn insert_fixup(&mut self, mut node: NodePtr<K, V>) {
        let mut parent;
        let mut gparent;

        while node.parent().is_red_color() {
            parent = node.parent();
            gparent = parent.parent();
            if parent == gparent.left() {
                let mut uncle = gparent.right();
                if !uncle.is_null() && uncle.is_red_color() {
                    uncle.set_color(RBColor::Black);
                    parent.set_color(RBColor::Black);
                    gparent.set_color(RBColor::Red);
                    node = gparent;
                    continue
                }

                if parent.right() == node {
                    self.left_rotate(parent);
                    let temp = parent;
                    parent = node;
                    node = temp;
                }

                parent.set_color(RBColor::Black);
                gparent.set_color(RBColor::Red);
                self.right_rotate(gparent);
            } else {
                let mut uncle = gparent.left();
                if !uncle.is_null() && uncle.is_red_color() {
                    uncle.set_color(RBColor::Black);
                    parent.set_color(RBColor::Black);
                    gparent.set_color(RBColor::Red);
                    node = gparent;
                    continue
                }

                if parent.left() == node {
                    self.right_rotate(parent);
                    let temp = parent;
                    parent = node;
                    node = temp;
                }

                parent.set_color(RBColor::Black);
                gparent.set_color(RBColor::Red);
                self.left_rotate(gparent);
            }
        }
        self.root.set_color(RBColor::Black)
    }

    #[inline(always)]
    pub fn insert(&mut self, k: K, v: V) {
        self.len += 1;
        let mut node = NodePtr::new(k, v);
        let mut y = NodePtr::null();
        let mut x = self.root;

        while !x.is_null() {
            y = x;
            match node.cmp(&&mut x) {
                Ordering::Less => {
                    x = x.left();
                }
                _ => {
                    x = x.right();
                }
            };
        }
        node.set_parent(y);

        if y.is_null() {
            self.root = node;
        } else {
            match node.cmp(&&mut y) {
                Ordering::Less => {
                    y.set_left(node);
                }
                _ => {
                    y.set_right(node)
                }
            };
        }

        node.set_color(RBColor::Red);
        unsafe {
            self.insert_fixup(node);
        }
    }
    
    #[inline(always)]
    fn find_node(&self, k: &K) -> NodePtr<K, V> {
        if self.root.is_null() {
            return NodePtr::null()
        }
        let mut temp = &self.root;
        unsafe {
            loop {
                let next = match k.cmp(&(*temp.0).key) {
                    Ordering::Less => &mut (*temp.0).left,
                    Ordering::Greater => &mut (*temp.0).right,
                    Ordering::Equal => return *temp
                };
                if next.is_null() {
                    break
                }
                temp = next;
            }
        }
        NodePtr::null()
    }

    #[inline(always)]
    fn first_child(&self) -> NodePtr<K, V> {
        if self.root.is_null() {
            return NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.left().is_null() {
                temp = temp.left();
            }
            return temp
        }
    }

    #[inline(always)]
    fn last_child(&self) -> NodePtr<K, V> {
        if self.root.is_null() {
            return NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.right().is_null() {
                temp = temp.right();
            }
            return temp
        }
    }

    #[inline(always)]
    pub fn get_first(&self) -> Option<(&K, &V)> {
        let first = self.first_child();
        if first.is_null() {
            return None
        }
        unsafe { Some((&(*first.0).key, &(*first.0).value)) }
    }

    #[inline(always)]
    pub fn get_last(&self) -> Option<(&K, &V)> {
        let last = self.last_child();
        if last.is_null() {
            return None
        }
        unsafe { Some((&(*last.0).key, &(*last.0).value)) }
    }

    #[inline(always)]
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        let first = self.first_child();
        if first.is_null() {
            return None
        }
        unsafe { Some(self.delete(first)) }
    }

    #[inline(always)]
    pub fn pop_last(&mut self ) -> Option<(K, V)> {
        let last = self.last_child();
        if last.is_null() {
            return None
        }
        unsafe { Some(self.delete(last)) }
    }

    #[inline(always)]
    pub fn get_first_mut(&mut self) -> Option<(&K, &mut V)> {
        let first = self.first_child();
        if first.is_null() {
            return None
        }
        unsafe { Some((&(*first.0).key, &mut (*first.0).value)) }
    }

    #[inline(always)]
    pub fn get_last_mut(&mut self) -> Option<(&K, &mut V)> {
        let last = self.last_child();
        if last.is_null() {
            return None
        }
        unsafe { Some((&(*last.0).key, &mut (*last.0).value)) }
    }

    #[inline(always)]
    pub fn get(&self, k: &K) -> Option<&mut V> {
        let node = self.find_node(k);
        if node.is_null() {
            return None
        }

        unsafe { Some(&mut (*node.0).value) }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let node = self.find_node(k);
        if node.is_null() {
            return None
        }
        unsafe { Some(&mut (*node.0).value) }
    }

    #[inline(always)]
    pub fn contains_key(&self, k: &K) -> bool {
        let node = self.find_node(k);
        if node.is_null() {
            return false
        }
        true
    }

    #[inline(always)]
    fn clear_recurse(&mut self, current: NodePtr<K, V>) {
        if !current.is_null() {
            unsafe {
                self.clear_recurse(current.left());
                self.clear_recurse(current.right());
                drop(Box::from_raw(current.0));
            }
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        let root = self.root;
        self.root = NodePtr::null();
        self.clear_recurse(root);
    }

    #[inline(always)]
    pub fn fast_clear(&mut self) {
        self.root = NodePtr::null();
    }

    #[inline(always)]
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let node = self.find_node(k);
        if node.is_null() {
            return None
        }
        unsafe { Some(self.delete(node).1) }
    }

    #[inline(always)]
    unsafe fn delete_fixup(&mut self, mut node: NodePtr<K, V>, mut parent: NodePtr<K, V>) {
        let mut other;
        while node != self.root && node.is_black_color() {
            if parent.left() == node {
                other = parent.right();
                if other.is_red_color() {
                    other.set_color(RBColor::Black);
                    parent.set_color(RBColor::Red);
                    self.left_rotate(parent);
                    other = parent.right();
                }

                if other.left().is_black_color() && other.right().is_black_color() {
                    other.set_color(RBColor::Red);
                    node = parent;
                    parent = node.parent();
                } else {
                    if other.right().is_black_color() {
                        other.left().set_color(RBColor::Black);
                        other.set_color(RBColor::Red);
                        self.right_rotate(other);
                        other = parent.right();
                    }
                    other.set_color(parent.get_color());
                    parent.set_color(RBColor::Black);
                    other.right().set_color(RBColor::Black);
                    self.left_rotate(parent);
                    node = self.root;
                    break
                }
            } else {
                other = parent.left();
                if other.is_red_color() {
                    other.set_color(RBColor::Black);
                    parent.set_color(RBColor::Red);
                    self.right_rotate(parent);
                    other = parent.left();
                }

                if other.left().is_black_color() && other.right().is_black_color() {
                    other.set_color(RBColor::Red);
                    node = parent;
                    parent = node.parent();
                } else {
                    if other.left().is_black_color() {
                        other.right().set_color(RBColor::Black);
                        other.set_color(RBColor::Red);
                        self.left_rotate(other);
                        other = parent.left();
                    }
                    other.set_color(parent.get_color());
                    parent.set_color(RBColor::Black);
                    other.left().set_color(RBColor::Black);
                    self.right_rotate(parent);
                    node = self.root;
                    break
                }
            }
        }
        node.set_color(RBColor::Black)
    }

    #[inline(always)]
    unsafe fn delete(&mut self, node: NodePtr<K, V>) -> (K, V) {
        let mut child;
        let mut parent;
        let color;

        self.len -= 1;
        if !(node.left().is_null() || node.right().is_null()) {
            let mut replace = node.right().min_node();
            if node == self.root {
                self.root = replace;
            } else {
                if node.parent().left() == node {
                    node.parent().set_left(replace);
                } else {
                    node.parent().set_right(replace);
                }
            }

            child = replace.right();
            parent = replace.parent();
            color = replace.get_color();
            if parent == node {
                parent = replace;
            } else {
                if !child.is_null() {
                    child.set_parent(parent);
                }
                parent.set_left(child);
                replace.set_right(node.right());
                node.right().set_parent(replace);
            }

            replace.set_parent(node.parent());
            replace.set_color(node.get_color());
            replace.set_left(node.left());
            node.left().set_parent(replace);

            if color == RBColor::Black {
                self.delete_fixup(child, parent);
            }

            let obj = Box::from_raw(node.0);
            return obj.pair();
        }

        if !node.left().is_null() {
            child = node.left();
        } else {
            child = node.right();
        }

        parent = node.parent();
        color = node.get_color();
        if !child.is_null() {
            child.set_parent(parent)
        }

        if self.root == node {
            self.root = child
        } else {
            if parent.left() == node {
                parent.set_left(child);
            } else {
                parent.set_right(child);
            }
        }

        if color == RBColor::Black {
            self.delete_fixup(child, parent);
        }

        let obj = Box::from_raw(node.0);
        return obj.pair()
    }
}