// Copyright (c) 2018 Ministerio de Fomento
//                    Instituto de Ciencias de la Construcción Eduardo Torroja (IETcc-CSIC)

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Author(s): Rafael Villar Burke <pachi@ietcc.csic.es>

use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

/// Energy pairs representing renewable and non renewable energy quantities or factors.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RenNrenCo2 {
    /// Renewable energy or factor
    #[serde(serialize_with = "round_serialize_3")]
    pub ren: f32,
    /// Non Renewable energy or factor
    #[serde(serialize_with = "round_serialize_3")]
    pub nren: f32,
    /// Non Renewable energy or factor
    #[serde(serialize_with = "round_serialize_3")]
    pub co2: f32,
}

fn round_serialize_3<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_f32( (x * 1000.0).round() / 1000.0)
}

impl RenNrenCo2 {
    /// Default constructor -> { ren: 0.0, nren: 0.0 }
    pub fn new() -> Self {
        Default::default()
    }

    /// Total renewable + non renewable energy
    pub fn tot(self) -> f32 {
        self.ren + self.nren
    }

    /// Renewable energy ratio
    pub fn rer(self) -> f32 {
        let tot = self.tot();
        if tot == 0.0 {
            0.0
        } else {
            self.ren / tot
        }
    }
}

impl fmt::Display for RenNrenCo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ren: {:.3}, nren: {:.3}, co2: {:.3} }}", self.ren, self.nren, self.co2)
    }
}

// The insane amount of boilerplate for ops would be simplified with the implementation
// of the Eye of Sauron in Rustc:
// - https://github.com/arielb1/rfcs/blob/df42b1df220d27876976b54dc93cdcb0b592cad3/text/0000-eye-of-sauron.md
// - https://github.com/rust-lang/rust/issues/44762

// Implement addition
impl Add for RenNrenCo2 {
    type Output = RenNrenCo2;

    fn add(self, other: RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren + other.ren,
            nren: self.nren + other.nren,
            co2: self.co2 + other.co2,
        }
    }
}

impl<'a> Add for &'a RenNrenCo2 {
    type Output = RenNrenCo2;

    fn add(self, other: &RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren + other.ren,
            nren: self.nren + other.nren,
            co2: self.co2 + other.co2,
        }
    }
}

// Implement +=
impl AddAssign for RenNrenCo2 {
    fn add_assign(&mut self, other: RenNrenCo2) {
        *self = RenNrenCo2 {
            ren: self.ren + other.ren,
            nren: self.nren + other.nren,
            co2: self.co2 + other.co2,
        };
    }
}

// Implement substraction
impl Sub for RenNrenCo2 {
    type Output = RenNrenCo2;

    fn sub(self, other: RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren - other.ren,
            nren: self.nren - other.nren,
            co2: self.co2 - other.co2,
        }
    }
}

impl<'a> Sub for &'a RenNrenCo2 {
    type Output = RenNrenCo2;

    fn sub(self, other: &RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren - other.ren,
            nren: self.nren - other.nren,
            co2: self.co2 - other.co2,
        }
    }
}

// Implement -=
impl SubAssign for RenNrenCo2 {
    fn sub_assign(&mut self, other: RenNrenCo2) {
        *self = RenNrenCo2 {
            ren: self.ren - other.ren,
            nren: self.nren - other.nren,
            co2: self.co2 - other.co2,
        };
    }
}

// Implement multiplication by a f32
// rennren * f32
impl Mul<f32> for RenNrenCo2 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: f32) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren * rhs,
            nren: self.nren * rhs,
            co2: self.co2 * rhs,
        }
    }
}

// rennren * &f32
impl<'a> Mul<&'a f32> for RenNrenCo2 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: &f32) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren * rhs,
            nren: self.nren * rhs,
            co2: self.co2 * rhs,
        }
    }
}

// &rennren * f32
impl<'a> Mul<f32> for &'a RenNrenCo2 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: f32) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self.ren * rhs,
            nren: self.nren * rhs,
            co2: self.co2 * rhs,
        }
    }
}

// TODO: &rennren * &f32 -> impl<'a, 'b> Mul<&'b f32> for &'a RenNRenPair

// f32 * rennren
impl Mul<RenNrenCo2> for f32 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self * rhs.ren,
            nren: self * rhs.nren,
            co2: self * rhs.co2,
        }
    }
}

// &f32 * rennren
impl<'a> Mul<RenNrenCo2> for &'a f32 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self * rhs.ren,
            nren: self * rhs.nren,
            co2: self * rhs.co2,
        }
    }
}

// f32 * &rennren
impl<'a> Mul<&'a RenNrenCo2> for f32 {
    type Output = RenNrenCo2;

    fn mul(self, rhs: &RenNrenCo2) -> RenNrenCo2 {
        RenNrenCo2 {
            ren: self * rhs.ren,
            nren: self * rhs.nren,
            co2: self * rhs.co2,
        }
    }
}

// TODO: &f32 * &rennren -> impl<'a, 'b> Mul<&'b RenNRenPair> for &'a f32

// Implement RenNren *= f32
impl MulAssign<f32> for RenNrenCo2 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = RenNrenCo2 {
            ren: self.ren * rhs,
            nren: self.nren * rhs,
            co2: self.co2 * rhs,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        assert_eq!(
            RenNrenCo2 {
                ren: 3.0,
                nren: 3.0,
                co2: 3.0
            },
            RenNrenCo2 {
                ren: 1.0,
                nren: 0.0,
                co2: 2.0
            } + RenNrenCo2 {
                ren: 2.0,
                nren: 3.0,
                co2: 1.0
            }
        );
        assert_eq!(
            RenNrenCo2 {
                ren: 3.0,
                nren: 3.0,
                co2: 3.0
            },
            {
                let mut a = RenNrenCo2 {
                    ren: 1.0,
                    nren: 0.0,
                    co2: 2.0
                };
                a += RenNrenCo2 {
                    ren: 2.0,
                    nren: 3.0,
                    co2: 1.0
                };
                a
            }
        );
    }
    #[test]
    fn sub() {
        assert_eq!(
            RenNrenCo2 {
                ren: -1.0,
                nren: -3.0,
                co2: 1.0
            },
            RenNrenCo2 {
                ren: 1.0,
                nren: 0.0,
                co2: 2.0
            } - RenNrenCo2 {
                ren: 2.0,
                nren: 3.0,
                co2: 1.0
            }
        );
        assert_eq!(
            RenNrenCo2 {
                ren: -1.0,
                nren: -3.0,
                co2: 1.0
            },
            {
                let mut a = RenNrenCo2 {
                    ren: 1.0,
                    nren: 0.0,
                    co2: 2.0
                };
                a -= RenNrenCo2 {
                    ren: 2.0,
                    nren: 3.0,
                    co2: 1.0
                };
                a
            }
        );
    }
    #[test]
    fn display() {
        assert_eq!(
            format!(
                "{}",
                RenNrenCo2 {
                    ren: 1.0,
                    nren: 0.0,
                    co2: 2.0
                }
            ),
            "{ ren: 1.000, nren: 0.000, co2: 2.000 }"
        );
    }
    #[test]
    fn mul() {
        assert_eq!(
            RenNrenCo2 {
                ren: 2.2,
                nren: 4.4,
                co2: 2.0
            },
            2.0 * RenNrenCo2 {
                ren: 1.1,
                nren: 2.2,
                co2: 1.0
            }
        );
        assert_eq!(
            RenNrenCo2 {
                ren: 2.2,
                nren: 4.4,
                co2: 2.0
            },
            {
                let mut a = RenNrenCo2 {
                    ren: 1.1,
                    nren: 2.2,
                    co2: 1.0
                };
                a *= 2.0;
                a
            }
        );
    }
}