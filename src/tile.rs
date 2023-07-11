use num_derive::FromPrimitive;
use rand::distributions::{Distribution, Standard};
use rand::Rng;

/// The number of [`Tile`] variants. 36 tiles from 6 colors and 6 shapes.
pub const TILES_LEN: usize = Color::COLORS_LEN * Shape::SHAPES_LEN;

/// Describes a tile with [`Color`] and [`Shape`] in a game.
pub type Tile = (Color, Shape);

/// # Returns
///
/// An array of all [`Tile`] variants in color then shape order.
#[inline]
pub fn tiles() -> [Tile; TILES_LEN] {
    [
        (Color::Red, Shape::Circle),
        (Color::Red, Shape::Clover),
        (Color::Red, Shape::Diamond),
        (Color::Red, Shape::Square),
        (Color::Red, Shape::Starburst),
        (Color::Red, Shape::X),
        (Color::Orange, Shape::Circle),
        (Color::Orange, Shape::Clover),
        (Color::Orange, Shape::Diamond),
        (Color::Orange, Shape::Square),
        (Color::Orange, Shape::Starburst),
        (Color::Orange, Shape::X),
        (Color::Yellow, Shape::Circle),
        (Color::Yellow, Shape::Clover),
        (Color::Yellow, Shape::Diamond),
        (Color::Yellow, Shape::Square),
        (Color::Yellow, Shape::Starburst),
        (Color::Yellow, Shape::X),
        (Color::Green, Shape::Circle),
        (Color::Green, Shape::Clover),
        (Color::Green, Shape::Diamond),
        (Color::Green, Shape::Square),
        (Color::Green, Shape::Starburst),
        (Color::Green, Shape::X),
        (Color::Blue, Shape::Circle),
        (Color::Blue, Shape::Clover),
        (Color::Blue, Shape::Diamond),
        (Color::Blue, Shape::Square),
        (Color::Blue, Shape::Starburst),
        (Color::Blue, Shape::X),
        (Color::Purple, Shape::Circle),
        (Color::Purple, Shape::Clover),
        (Color::Purple, Shape::Diamond),
        (Color::Purple, Shape::Square),
        (Color::Purple, Shape::Starburst),
        (Color::Purple, Shape::X),
    ]
}

/// Describes the color on a [`Tile`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
pub enum Color {
    /// `0`.
    Red = 0,
    /// `1`.
    Orange = 1,
    /// `2`.
    Yellow = 2,
    /// `3`.
    Green = 3,
    /// `4`.
    Blue = 4,
    /// `5`.
    Purple = 5,
}

impl Color {
    /// The number of [`Color`] variants. 6 colors.
    pub const COLORS_LEN: usize = 6;

    /// # Returns
    ///
    /// An array of all [`Color`] variants in order.
    #[inline]
    pub fn colors() -> [Color; Color::COLORS_LEN] {
        [
            Color::Red,
            Color::Orange,
            Color::Yellow,
            Color::Green,
            Color::Blue,
            Color::Purple,
        ]
    }
}

impl Distribution<Color> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Color {
        let index = rng.gen_range(0..Color::COLORS_LEN);
        num::FromPrimitive::from_usize(index).unwrap_or_else(|| {
            dbg!(index, Color::COLORS_LEN);
            unreachable!(
                "index ({:?}) should be matched since colors cover all indexes \
                in range 0..Color::COLORS_LEN (0..{:?}).",
                index,
                Color::COLORS_LEN
            );
        })
    }
}

/// Describes the shape on a [`Tile`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[allow(missing_docs)]
pub enum Shape {
    /// `0`.
    Circle = 0,
    /// `1`.
    Clover = 1,
    /// `2`.
    Diamond = 2,
    /// `3`.
    Square = 3,
    /// `4`.
    Starburst = 4,
    /// `5`.
    X = 5,
}

impl Shape {
    /// The number of [`Shape`] variants. 6 shapes.
    pub const SHAPES_LEN: usize = 6;

    /// # Returns
    ///
    /// An array of all [`Shape`] variants in order.
    #[inline]
    pub fn shapes() -> [Shape; Shape::SHAPES_LEN] {
        [
            Shape::Circle,
            Shape::Clover,
            Shape::Diamond,
            Shape::Square,
            Shape::Starburst,
            Shape::X,
        ]
    }
}

impl Distribution<Shape> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Shape {
        let index = rng.gen_range(0..Shape::SHAPES_LEN);
        num::FromPrimitive::from_usize(index).unwrap_or_else(|| {
            dbg!(index, Shape::SHAPES_LEN);
            unreachable!(
                "index ({:?}) should be matched since shapes cover all indexes \
                in range 0..Shape::SHAPES_LEN (0..{:?}).",
                index,
                Shape::SHAPES_LEN
            );
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn tiles_len() {
        assert_eq!(TILES_LEN, Color::COLORS_LEN * Shape::SHAPES_LEN);
        assert_eq!(TILES_LEN, tiles().len());
    }

    #[test]
    fn tiles_no_duplicates() {
        assert_eq!(0, tiles().into_iter().duplicates().count());
    }

    #[test]
    fn colors() {
        assert_eq!(Color::COLORS_LEN, Color::colors().len());
    }

    #[test]
    fn colors_no_duplicates() {
        assert_eq!(0, Color::colors().into_iter().duplicates().count());
    }

    #[test]
    fn color_as_usize() {
        for (index, color) in Color::colors().into_iter().enumerate() {
            assert_eq!(index, color as usize);
        }
    }

    #[test]
    fn count_colors() {
        let mut counts = [0; Color::COLORS_LEN];
        for color in Color::colors() {
            counts[color as usize] += 1;
        }

        for count in counts {
            assert_eq!(1, count);
        }
    }

    #[test]
    fn shapes() {
        assert_eq!(Shape::SHAPES_LEN, Shape::shapes().len());
    }

    #[test]
    fn shapes_no_duplicates() {
        assert_eq!(0, Shape::shapes().into_iter().duplicates().count());
    }

    #[test]
    fn shape_as_usize() {
        for (index, shape) in Shape::shapes().into_iter().enumerate() {
            assert_eq!(index, shape as usize);
        }
    }

    #[test]
    fn count_shapes() {
        let mut counts = [0; Shape::SHAPES_LEN];
        for shape in Shape::shapes() {
            counts[shape as usize] += 1;
        }
        for count in counts {
            assert_eq!(1, count);
        }
    }
}
