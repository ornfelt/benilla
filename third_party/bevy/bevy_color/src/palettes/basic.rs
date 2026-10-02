//! Named colors from the CSS1 specification, also known as
//! [basic colors](https://en.wikipedia.org/wiki/Web_colors#Basic_colors).
//! This is the same set of colors used in the
//! [VGA graphics standard](https://en.wikipedia.org/wiki/Video_Graphics_Array).

use crate::Srgba;
/// <div style="background-color: #0000FF; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);
/// <div style="background-color: #008000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const GREEN: Srgba = Srgba::rgb(0.0, 0.5019608, 0.0);
/// <div style="background-color: #FF0000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
