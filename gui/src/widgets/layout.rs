//! Layout managers for automatic widget positioning

use super::Rect;

/// Trait for layout algorithms
///
/// A layout computes the bounding rectangles for a list of widgets
/// given the available space.
pub trait Layout: Send {
    /// Compute widget bounds given available space
    ///
    /// Returns a Vec of Rects, one for each widget in the container.
    /// The order matches the order widgets were added to the container.
    fn compute_bounds(&self, available_width: f32, available_height: f32) -> Vec<Rect>;

    /// Add a widget's preferred size to the layout
    ///
    /// Some layouts (like Manual) ignore this, others (like Grid/Flex) use it
    fn add_widget(&mut self, width: f32, height: f32);
}

/// Manual layout - explicit positioning
///
/// Each widget is positioned at a specific (x, y) coordinate.
/// Useful for custom UIs where you need precise control.
pub struct ManualLayout {
    positions: Vec<(f32, f32)>,
}

impl ManualLayout {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
        }
    }

    /// Add a widget at the specified position
    pub fn add_widget_at(&mut self, x: f32, y: f32, _width: f32, _height: f32) {
        self.positions.push((x, y));
    }
}

impl Default for ManualLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for ManualLayout {
    fn compute_bounds(&self, _available_width: f32, _available_height: f32) -> Vec<Rect> {
        // For manual layout, we need the widget sizes stored
        // For now, return empty rects at the positions
        // This will be refined when we integrate with the container
        self.positions
            .iter()
            .map(|(x, y)| Rect::new(*x, *y, 0.0, 0.0))
            .collect()
    }

    fn add_widget(&mut self, width: f32, height: f32) {
        // For manual layout, widgets must be added with explicit positions
        // This is a fallback that stacks widgets vertically
        let y = self.positions.len() as f32 * (height + 10.0);
        self.add_widget_at(10.0, y, width, height);
    }
}

/// Grid layout - automatic grid placement
///
/// Widgets are placed in a grid with configurable rows, columns, and spacing.
pub struct GridLayout {
    rows: usize,
    cols: usize,
    spacing: f32,
    padding: f32,
    widget_sizes: Vec<(f32, f32)>,
}

impl GridLayout {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            spacing: 10.0,
            padding: 10.0,
            widget_sizes: Vec::new(),
        }
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl Layout for GridLayout {
    fn compute_bounds(&self, available_width: f32, available_height: f32) -> Vec<Rect> {
        let mut rects = Vec::new();

        // Calculate cell dimensions
        let usable_width = available_width - 2.0 * self.padding;
        let usable_height = available_height - 2.0 * self.padding;

        let cell_width =
            (usable_width - (self.cols as f32 - 1.0) * self.spacing) / self.cols as f32;
        let cell_height =
            (usable_height - (self.rows as f32 - 1.0) * self.spacing) / self.rows as f32;

        // Place widgets in grid cells
        for (i, (w, h)) in self.widget_sizes.iter().enumerate() {
            let row = i / self.cols;
            let col = i % self.cols;

            if row >= self.rows {
                break; // Don't overflow the grid
            }

            let x = self.padding + col as f32 * (cell_width + self.spacing);
            let y = self.padding + row as f32 * (cell_height + self.spacing);

            // Center widget in cell if it's smaller
            let x = x + (cell_width - w).max(0.0) / 2.0;
            let y = y + (cell_height - h).max(0.0) / 2.0;

            rects.push(Rect::new(x, y, *w, *h));
        }

        rects
    }

    fn add_widget(&mut self, width: f32, height: f32) {
        self.widget_sizes.push((width, height));
    }
}

/// Flexbox-like layout
///
/// CSS Flexbox-inspired layout with direction, justification, and alignment.
pub struct FlexLayout {
    direction: FlexDirection,
    justify: JustifyContent,
    align: AlignItems,
    spacing: f32,
    padding: f32,
    widget_sizes: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
}

impl FlexLayout {
    pub fn new(direction: FlexDirection) -> Self {
        Self {
            direction,
            justify: JustifyContent::Start,
            align: AlignItems::Start,
            spacing: 10.0,
            padding: 10.0,
            widget_sizes: Vec::new(),
        }
    }

    pub fn with_justify(mut self, justify: JustifyContent) -> Self {
        self.justify = justify;
        self
    }

    pub fn with_align(mut self, align: AlignItems) -> Self {
        self.align = align;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl Layout for FlexLayout {
    fn compute_bounds(&self, available_width: f32, available_height: f32) -> Vec<Rect> {
        let mut rects = Vec::new();

        if self.widget_sizes.is_empty() {
            return rects;
        }

        let usable_width = available_width - 2.0 * self.padding;
        let usable_height = available_height - 2.0 * self.padding;

        match self.direction {
            FlexDirection::Row => {
                let total_widget_width: f32 = self.widget_sizes.iter().map(|(w, _)| w).sum();
                let total_spacing = (self.widget_sizes.len() - 1) as f32 * self.spacing;

                let mut x_offset = self.padding;

                // Calculate starting offset based on justify
                match self.justify {
                    JustifyContent::Start => {}
                    JustifyContent::End => {
                        x_offset += usable_width - total_widget_width - total_spacing;
                    }
                    JustifyContent::Center => {
                        x_offset += (usable_width - total_widget_width - total_spacing) / 2.0;
                    }
                    JustifyContent::SpaceBetween | JustifyContent::SpaceAround => {
                        // Handled in loop
                    }
                }

                let extra_spacing = match self.justify {
                    JustifyContent::SpaceBetween => {
                        if self.widget_sizes.len() > 1 {
                            (usable_width - total_widget_width)
                                / (self.widget_sizes.len() - 1) as f32
                        } else {
                            0.0
                        }
                    }
                    JustifyContent::SpaceAround => {
                        let total_gap = usable_width - total_widget_width;
                        let gap = total_gap / self.widget_sizes.len() as f32;
                        x_offset += gap / 2.0;
                        gap
                    }
                    _ => self.spacing,
                };

                for (w, h) in &self.widget_sizes {
                    let y = match self.align {
                        AlignItems::Start => self.padding,
                        AlignItems::End => self.padding + usable_height - h,
                        AlignItems::Center => self.padding + (usable_height - h) / 2.0,
                        AlignItems::Stretch => self.padding,
                    };

                    let height = if matches!(self.align, AlignItems::Stretch) {
                        usable_height
                    } else {
                        *h
                    };

                    rects.push(Rect::new(x_offset, y, *w, height));
                    x_offset += w + extra_spacing;
                }
            }
            FlexDirection::Column => {
                let total_widget_height: f32 = self.widget_sizes.iter().map(|(_, h)| h).sum();
                let total_spacing = (self.widget_sizes.len() - 1) as f32 * self.spacing;

                let mut y_offset = self.padding;

                // Calculate starting offset based on justify
                match self.justify {
                    JustifyContent::Start => {}
                    JustifyContent::End => {
                        y_offset += usable_height - total_widget_height - total_spacing;
                    }
                    JustifyContent::Center => {
                        y_offset += (usable_height - total_widget_height - total_spacing) / 2.0;
                    }
                    JustifyContent::SpaceBetween | JustifyContent::SpaceAround => {
                        // Handled in loop
                    }
                }

                let extra_spacing = match self.justify {
                    JustifyContent::SpaceBetween => {
                        if self.widget_sizes.len() > 1 {
                            (usable_height - total_widget_height)
                                / (self.widget_sizes.len() - 1) as f32
                        } else {
                            0.0
                        }
                    }
                    JustifyContent::SpaceAround => {
                        let total_gap = usable_height - total_widget_height;
                        let gap = total_gap / self.widget_sizes.len() as f32;
                        y_offset += gap / 2.0;
                        gap
                    }
                    _ => self.spacing,
                };

                for (w, h) in &self.widget_sizes {
                    let x = match self.align {
                        AlignItems::Start => self.padding,
                        AlignItems::End => self.padding + usable_width - w,
                        AlignItems::Center => self.padding + (usable_width - w) / 2.0,
                        AlignItems::Stretch => self.padding,
                    };

                    let width = if matches!(self.align, AlignItems::Stretch) {
                        usable_width
                    } else {
                        *w
                    };

                    rects.push(Rect::new(x, y_offset, width, *h));
                    y_offset += h + extra_spacing;
                }
            }
        }

        rects
    }

    fn add_widget(&mut self, width: f32, height: f32) {
        self.widget_sizes.push((width, height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_layout_2x2() {
        let mut layout = GridLayout::new(2, 2).with_spacing(10.0).with_padding(5.0);

        // Add 4 widgets
        layout.add_widget(50.0, 30.0);
        layout.add_widget(50.0, 30.0);
        layout.add_widget(50.0, 30.0);
        layout.add_widget(50.0, 30.0);

        let rects = layout.compute_bounds(200.0, 150.0);
        assert_eq!(rects.len(), 4);

        // First widget should be at padding position
        assert!(rects[0].x >= 5.0);
        assert!(rects[0].y >= 5.0);
    }

    #[test]
    fn test_flex_row_center() {
        let mut layout = FlexLayout::new(FlexDirection::Row)
            .with_justify(JustifyContent::Center)
            .with_padding(0.0)
            .with_spacing(10.0);

        layout.add_widget(50.0, 30.0);
        layout.add_widget(60.0, 30.0);

        let rects = layout.compute_bounds(200.0, 100.0);
        assert_eq!(rects.len(), 2);

        // Total widget width: 50 + 60 = 110
        // Total spacing: 10
        // Total: 120
        // Centering in 200px: (200 - 120) / 2 = 40
        assert!((rects[0].x - 40.0).abs() < 1.0);
    }

    #[test]
    fn test_flex_column_space_between() {
        let mut layout = FlexLayout::new(FlexDirection::Column)
            .with_justify(JustifyContent::SpaceBetween)
            .with_padding(0.0);

        layout.add_widget(50.0, 20.0);
        layout.add_widget(50.0, 20.0);
        layout.add_widget(50.0, 20.0);

        let rects = layout.compute_bounds(100.0, 100.0);
        assert_eq!(rects.len(), 3);

        // Total widget height: 60
        // Available: 100
        // Gap: (100 - 60) / 2 = 20
        assert!((rects[0].y - 0.0).abs() < 1.0);
        assert!((rects[1].y - 40.0).abs() < 1.0);
        assert!((rects[2].y - 80.0).abs() < 1.0);
    }
}
