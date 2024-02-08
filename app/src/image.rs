use boat_journey_game::MenuImage;
use chargrid::prelude::*;
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
}

#[derive(Clone, Copy)]
enum ImageName {}

impl ImageName {
    const fn data(self) -> &'static [u8] {
        match self {}
    }

    fn load_grid(self) -> Image {
        let grid = bincode::deserialize::<Grid<RenderCell>>(self.data()).unwrap();
        Image { grid }
    }
}

pub struct Images {}

impl Images {
    pub fn new() -> Self {
        Self {}
    }

    pub fn image_from_menu_image(&self, menu_image: MenuImage) -> &Image {
        match menu_image {}
    }
}
