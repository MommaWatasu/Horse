use alloc::{
    sync::Arc,
    vec::Vec,
    vec
};
use core::cell::RefCell;
use crate::{
    error,
    graphics::{
        Coord,
        FrameBufferWriter,
        PixelWriter
    },
    FrameBuffer,
    FrameBufferConfig,
    window::Window
};
use spin::Once;

pub static mut LAYER_MANAGER: Once<LayerManager> = Once::new();

#[derive(Clone, Default, PartialEq)]
pub struct Layer {
    id: u32,
    pos: Coord,
    pub window: Arc<Window>
}

impl Layer {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            pos: Default::default(),
            window: Default::default()
        }
    }

    pub fn id(&self) -> u32 { self.id }

    pub fn set_window(&mut self, window: Arc<Window>) -> &mut Self {
        self.window = window;
        return self
    }

    pub fn get_window(&self) -> Arc<Window> { self.window.clone() }

    pub fn move_absolute(&mut self, pos: Coord) -> &mut Self {
        self.pos = pos;
        return self
    }

    pub fn move_relative(&mut self, pos_diff: Coord) -> &mut Self {
        self.pos += pos_diff;
        return self
    }

    pub fn draw_to(&self, fb: &mut FrameBuffer) {
        self.window.draw_to(fb, self.pos);
    }
}

#[derive(PartialEq)]
pub enum LayerHeight {
    Hide,
    Height(usize)
}

impl LayerHeight {
    pub fn is_hide(&self) -> bool { self == &Self::Hide }
    pub fn height(&self) -> Option<usize> {
        match self {
            Self::Height(height) => Some(*height),
            Self::Hide => None
        }
    }
}

pub struct LayerManager {
    fb: FrameBuffer,
    layers: Vec<Arc<RefCell<Layer>>>,
    layer_stack: Vec<Arc<RefCell<Layer>>>,
    layer_id: u32
}

impl LayerManager {
    pub fn new(fb_config: FrameBufferConfig) -> Self {
        return Self {
            fb: FrameBuffer::new(fb_config),
            layers: vec![],
            layer_stack: vec![],
            layer_id: 0
        }
    }

    pub fn new_layer(&mut self) -> Arc<RefCell<Layer>> {
        self.layer_id += 1;
        let layer = Arc::new(RefCell::new(Layer::new(self.layer_id)));
        self.layers.push(layer.clone());
        return layer.clone()
    }

    pub fn draw(&mut self) {
        for layer in &self.layer_stack {
            layer.borrow().draw_to(&mut self.fb);
        }
    }

    pub fn move_absolute(&mut self, id: u32, new_position: Coord) -> Result<(), ()> {
        self.find_layer(id)?.borrow_mut().move_absolute(new_position);
        Ok(())
    }

    pub fn move_relative(&mut self, id: u32, pos_diff: Coord) -> Result<(), ()> {
        self.find_layer(id)?.borrow_mut().move_relative(pos_diff);
        Ok(())
    }

    pub fn up_down(&mut self, id: u32, height: LayerHeight) -> Result<(), ()> {
        if height.is_hide() {
            self.hide(id)?;
            return Ok(())
        }
        let mut new_height = height.height().unwrap();

        let layer = self.find_layer(id)?;
        let old_pos = self.find_ord(id);
        
        if old_pos == None {
            self.layer_stack.insert(new_height, layer);
            return Ok(())
        }
        if new_height >= self.layer_stack.len() {
            new_height = self.layer_stack.len() - 1;
        }
        self.layer_stack.remove(old_pos.unwrap());
        self.layer_stack.insert(new_height, layer);
        Ok(())
    }

    pub fn hide(&mut self, id: u32) -> Result<(), ()> {
        if let Some(pos) = self.find_ord(id) {
            self.layer_stack.remove(pos);
        } else {
            return Err(())
        }
        Ok(())
    }

    fn find_layer(&self, id: u32) -> Result<Arc<RefCell<Layer>>, ()> {
        match self.layers.iter().find(|x| x.borrow().id() == id) {
            Some(layer) => Ok(layer.clone()),
            None => {
                error!("the layer isn't available");
                Err(())
            }
        }
    }

    fn find_ord(&self, id: u32) -> Option<usize> {
        self.layer_stack.iter().position(|x| x.borrow().id() == id)
    }
}