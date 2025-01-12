//! Transverse Mercator, following to Bowring (1989)
use super::*;

// ----- F O R W A R D -----------------------------------------------------------------

// Forward transverse mercator, following Bowring (1989)
fn fwd(op: &Op, _ctx: &dyn Context, operands: &mut [Coord]) -> Result<usize, Error> {
    let ellps = op.params.ellps[0];
    let eps = ellps.second_eccentricity_squared();
    let lat_0 = op.params.lat[0];
    let lon_0 = op.params.lon[0];
    let x_0 = op.params.x[0];
    let y_0 = op.params.y[0];
    let k_0 = op.params.k[0];

    let mut successes = 0_usize;
    for coord in operands {
        let lat = coord[1] + lat_0;
        let (s, c) = lat.sin_cos();
        let cc = c * c;
        let ss = s * s;

        let dlon = coord[0] - lon_0;
        let oo = dlon * dlon;

        #[allow(non_snake_case)]
        let N = ellps.prime_vertical_radius_of_curvature(lat);
        let z = eps * dlon.powi(3) * c.powi(5) / 6.;
        let sd2 = (dlon / 2.).sin();

        let theta_2 = (2. * s * c * sd2 * sd2).atan2(ss + cc * dlon.cos());

        // Easting
        let sd = dlon.sin();
        coord[0] = x_0 + k_0 * N * ((c * sd).atanh() + z * (1. + oo * (36. * cc - 29.) / 10.));

        // Northing
        let m = ellps.meridional_distance(lat, Fwd);
        let znos4 = z * N * dlon * s / 4.;
        let ecc = 4. * eps * cc;
        coord[1] = y_0 + k_0 * (m + N * theta_2 + znos4 * (9. + ecc + oo * (20. * cc - 11.)));
        successes += 1;
    }

    Ok(successes)
}

// ----- I N V E R S E -----------------------------------------------------------------

// Inverse transverse mercator, following Bowring (1989)
fn inv(op: &Op, _ctx: &dyn Context, operands: &mut [Coord]) -> Result<usize, Error> {
    let ellps = op.params.ellps[0];
    let eps = ellps.second_eccentricity_squared();
    let lat_0 = op.params.lat[0];
    let lon_0 = op.params.lon[0];
    let x_0 = op.params.x[0];
    let y_0 = op.params.y[0];
    let k_0 = op.params.k[0];

    let mut successes = 0_usize;
    for coord in operands {
        // Footpoint latitude, i.e. the latitude of a point on the central meridian
        // having the same northing as the point of interest
        let lat = ellps.meridional_distance((coord[1] - y_0) / k_0, Inv);
        let t = lat.tan();
        let c = lat.cos();
        let cc = c * c;
        #[allow(non_snake_case)]
        let N = ellps.prime_vertical_radius_of_curvature(lat);
        let x = (coord[0] - x_0) / (k_0 * N);
        let xx = x * x;
        let theta_4 = x.sinh().atan2(c);
        let theta_5 = (t * theta_4.cos()).atan();

        // Latitude
        let xet = xx * xx * eps * t / 24.;
        coord[1] = lat_0 + (1. + cc * eps) * (theta_5 - xet * (9. - 10. * cc)) - eps * cc * lat;

        // Longitude
        let approx = lon_0 + theta_4;
        let coef = eps / 60. * xx * x * c;
        coord[0] = approx - coef * (10. - 4. * xx / cc + xx * cc);

        successes += 1;
    }

    Ok(successes)
}

// ----- C O N S T R U C T O R ---------------------------------------------------------

#[rustfmt::skip]
pub const GAMUT: [OpParameter; 7] = [
    OpParameter::Flag { key: "inv" },
    OpParameter::Text { key: "ellps", default: Some("GRS80") },

    OpParameter::Real { key: "lat_0", default: Some(0_f64) },
    OpParameter::Real { key: "lon_0", default: Some(0_f64) },
    OpParameter::Real { key: "x_0",   default: Some(0_f64) },
    OpParameter::Real { key: "y_0",   default: Some(0_f64) },

    OpParameter::Real { key: "k_0",   default: Some(1_f64) },
];

pub fn new(parameters: &RawParameters, ctx: &dyn Context) -> Result<Op, Error> {
    Op::plain(parameters, InnerOp(fwd), InnerOp(inv), &GAMUT, ctx)
}

#[rustfmt::skip]
pub const UTM_GAMUT: [OpParameter; 3] = [
    OpParameter::Flag { key: "inv" },
    OpParameter::Text { key: "ellps", default: Some("GRS80") },
    OpParameter::Natural { key: "zone", default: None },
];

pub fn utm(parameters: &RawParameters, _ctx: &dyn Context) -> Result<Op, Error> {
    let def = &parameters.definition;
    let mut params = ParsedParameters::new(parameters, &UTM_GAMUT)?;

    // The UTM zone should be an integer between 1 and 60
    let zone = params.natural("zone")?;
    if !(1..61).contains(&zone) {
        return Err(Error::General(
            "UTM: 'zone' must be an integer in the interval 1..60",
        ));
    }

    // The scaling factor is 0.9996 by definition of UTM
    params.k[0] = 0.9996;

    // The center meridian is determined by the zone
    params.lon[0] = (-183. + 6. * zone as f64).to_radians();

    // The base parallel is by definition the equator
    params.lat[0] = 0.0;

    // The false easting is 500000 m by definition of UTM
    params.x[0] = 500000.0;

    // The false northing is 0 m by definition of UTM
    params.x[0] = 500000.0;

    let descriptor = OpDescriptor::new(def, InnerOp(fwd), Some(InnerOp(inv)));
    let steps = Vec::<Op>::new();
    let id = OpHandle::new();

    Ok(Op {
        descriptor,
        params,
        steps,
        id,
    })
}

// ----- T E S T S ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btmerc() -> Result<(), Error> {
        let ctx = Minimal::default();
        let definition = "btmerc k_0=0.9996 lon_0=9 x_0=500000";
        let op = Op::new(definition, &ctx)?;

        // Validation values from PROJ:
        // echo 12 55 0 0 | cct -d18 +proj=utm +zone=32 | clip
        #[rustfmt::skip]
        let geo = [
            Coord::geo( 55.,  12., 0., 0.),
            Coord::geo(-55.,  12., 0., 0.),
            Coord::geo( 55., -6., 0., 0.),
            Coord::geo(-55., -6., 0., 0.)
        ];

        #[rustfmt::skip]
        let projected = [
            Coord::raw( 691_875.632_139_661, 6_098_907.825_005_012, 0., 0.),
            Coord::raw( 691_875.632_139_661,-6_098_907.825_005_012, 0., 0.),
            Coord::raw(-455_673.814_189_040, 6_198_246.671_090_279, 0., 0.),
            Coord::raw(-455_673.814_189_040,-6_198_246.671_090_279, 0., 0.)
        ];

        let mut operands = geo.clone();
        op.apply(&ctx, &mut operands, Fwd)?;
        for i in 0..operands.len() {
            assert!(operands[i].hypot2(&projected[i]) < 5e-3);
        }

        op.apply(&ctx, &mut operands, Inv)?;
        for i in 0..operands.len() {
            assert!(operands[i].hypot2(&geo[i]) < 10e-8);
        }
        Ok(())
    }

    #[test]
    fn butm() -> Result<(), Error> {
        let ctx = Minimal::default();
        let definition = "butm zone=32";
        let op = Op::new(definition, &ctx)?;

        // Validation values from PROJ:
        // echo 12 55 0 0 | cct -d18 +proj=utm +zone=32 | clip
        #[rustfmt::skip]
        let geo = [
            Coord::geo( 55.,  12., 0., 0.),
            Coord::geo(-55.,  12., 0., 0.),
            Coord::geo( 55., -6., 0., 0.),
            Coord::geo(-55., -6., 0., 0.)
        ];

        #[rustfmt::skip]
        let projected = [
            Coord::raw( 691_875.632_139_661, 6_098_907.825_005_012, 0., 0.),
            Coord::raw( 691_875.632_139_661,-6_098_907.825_005_012, 0., 0.),
            Coord::raw(-455_673.814_189_040, 6_198_246.671_090_279, 0., 0.),
            Coord::raw(-455_673.814_189_040,-6_198_246.671_090_279, 0., 0.)
        ];

        let mut operands = geo.clone();
        op.apply(&ctx, &mut operands, Fwd)?;
        for i in 0..operands.len() {
            assert!(operands[i].hypot2(&projected[i]) < 5e-3);
        }

        op.apply(&ctx, &mut operands, Inv)?;
        for i in 0..operands.len() {
            assert!(operands[i].hypot2(&geo[i]) < 10e-8);
        }
        Ok(())
    }
}
