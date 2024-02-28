use chargrid::prelude::*;
use game::MenuImage;
use grid_2d::Grid;

pub struct Image {
    pub grid: Grid<RenderCell>,
}

impl Image {
    pub fn render(&self, ctx: Ctx, fb: &mut FrameBuffer) {
        for (coord, &cell) in self.grid.enumerate() {
            fb.set_cell_relative_to_ctx(ctx, coord, 0, cell);
        }
    }

    fn load(data: &[u8]) -> Self {
        Self {
            grid: bincode::deserialize::<Grid<RenderCell>>(data).unwrap(),
        }
    }
}

#[derive(Clone, Copy)]
enum ImageName {
    Placeholder,
}

impl ImageName {
    const fn data(self) -> &'static [u8] {
        use ImageName::*;
        match self {
            Placeholder => include_bytes!("images/placeholder.bin"),
        }
    }

    fn load(self) -> Image {
        Image::load(self.data())
    }
}

pub struct Images {
    pub placeholder: Image,
}

impl Images {
    pub fn new() -> Self {
        use ImageName::*;
        Self {
            placeholder: Placeholder.load(),
        }
    }

    pub fn image_from_menu_image(&self, menu_image: MenuImage) -> &Image {
        match menu_image {}
    }
}
