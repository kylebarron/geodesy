//! Lambert azimuthal equal area: EPSG coordinate operation method 9820, implemented
//! following [IOGP, 2019](crate::Bibliography::Iogp19), pp. 78-80
use super::*;
use std::f64::consts::FRAC_PI_2;
const EPS10: f64 = 1e-10;

// ----- C O M M O N -------------------------------------------------------------------

// ----- F O R W A R D -----------------------------------------------------------------

fn fwd(op: &Op, _ctx: &dyn Context, operands: &mut [Coord]) -> Result<usize, Error> {
    // Oblique aspect: [IOGP, 2019](crate::Bibliography::Iogp19), pp. 78-80
    let Ok(xi_0) = op.params.real("xi_0") else { return Ok(0) };
    let Ok(qp)   = op.params.real("qp")   else { return Ok(0) };
    let Ok(rq)   = op.params.real("rq")   else { return Ok(0) };
    let Ok(d)    = op.params.real("d")    else { return Ok(0) };

    let oblique = op.params.boolean("oblique");
    let north_polar = op.params.boolean("north_polar");
    let south_polar = op.params.boolean("south_polar");

    let lon_0 = op.params.lon(0);
    let x_0 = op.params.x(0);
    let y_0 = op.params.y(0);
    let ellps = op.params.ellps(0);
    let e = ellps.eccentricity();
    let a = ellps.semimajor_axis();

    let (sin_xi_0, cos_xi_0) = xi_0.sin_cos();

    let mut successes = 0_usize;

    // The polar aspects are fairly simple
    if north_polar || south_polar {
        for coord in operands {
            let sign = if north_polar { -1.0 } else { 1.0 };

            let lat = coord[1];
            let lon = coord[0];
            let (sin_lon, cos_lon) = (lon - lon_0).sin_cos();

            let q = qs(lat.sin(), e);
            let rho = a * (qp + sign * q).sqrt();

            coord[0] = x_0 + rho * sin_lon;
            coord[1] = y_0 + sign * rho * cos_lon;
            successes += 1;
        }
        return Ok(successes);
    }

    for coord in operands {
        let lon = coord[0];
        let lat = coord[1];
        let (sin_lon, cos_lon) = (lon - lon_0).sin_cos();

        // Authalic latitude, 𝜉
        let xi = (qs(lat.sin(), e) / qp).asin();

        let (sin_xi, cos_xi) = xi.sin_cos();
        let b = if oblique {
            let factor = 1.0 + sin_xi_0 * sin_xi + (cos_xi_0 * cos_xi * cos_lon);
            rq * (2.0 / factor).sqrt()
        } else {
            1.0
        };

        // Easting
        coord[0] = x_0 + (b * d) * (cos_xi * sin_lon);

        // Northing
        coord[1] = y_0 + (b / d) * (cos_xi_0 * sin_xi - sin_xi_0 * cos_xi * cos_lon);

        successes += 1;
    }

    Ok(successes)
}

// ----- I N V E R S E -----------------------------------------------------------------

fn inv(op: &Op, _ctx: &dyn Context, operands: &mut [Coord]) -> Result<usize, Error> {
    // Oblique aspect: [IOGP, 2019](crate::Bibliography::Iogp19), pp. 78-80
    let Ok(xi_0) = op.params.real("xi_0") else { return Ok(0) };
    let Ok(rq)   = op.params.real("rq")   else { return Ok(0) };
    let Ok(d)    = op.params.real("d")    else { return Ok(0) };
    let Ok(authalic)  = op.params.fourier_coefficients("authalic") else { return Ok(0) };

    let north_polar = op.params.boolean("north_polar");
    let south_polar = op.params.boolean("south_polar");

    let lon_0 = op.params.lon(0);
    let lat_0 = op.params.lat(0);
    let x_0 = op.params.x(0);
    let y_0 = op.params.y(0);

    let ellps = op.params.ellps(0);
    let a = ellps.semimajor_axis();
    let es = ellps.eccentricity_squared();
    let e = es.sqrt();

    let (sin_xi_0, cos_xi_0) = xi_0.sin_cos();

    let mut successes = 0_usize;

    // The polar aspects are not as simple as in the forward case
    if north_polar || south_polar {
        for coord in operands {
            let sign = if north_polar { -1.0 } else { 1.0 };

            let x = coord[0];
            let y = coord[1];
            let rho = (x - x_0).hypot(y - y_0);

            // The authalic latitude is a bit convoluted
            let denom = a * a * (1.0 - ((1.0 - es) / (2.0 * e)) * ((1.0 - e) / (1.0 + e)).ln());
            let xi = (-sign) * (1.0 - rho * rho / denom);

            coord[0] = lon_0 + (x - x_0).atan2(sign * (y - y_0));
            coord[1] = ellps.latitude_authalic_to_geographic(xi, &authalic);
            successes += 1;
        }
        return Ok(successes);
    }

    for coord in operands {
        let x = coord[0];
        let y = coord[1];
        let rho = ((x - x_0) / d).hypot(d * (y - y_0));
        // A bit of reality hardening ported from the PROJ implementation
        if rho < EPS10 {
            coord[0] = 0.0;
            coord[1] = lat_0;
            successes += 1;
            continue;
        }

        // Another case of PROJ reality hardening
        let asin_argument = 0.5 * rho / rq;
        if asin_argument.abs() > 1.0 {
            warn!("LAEA: ({}, {}) outside domain", x, y);
            continue;
        }

        let c = 2.0 * asin_argument.asin();
        let (sin_c, cos_c) = c.sin_cos();
        // The authalic latitude, 𝜉
        let xi = (cos_c * sin_xi_0 + (d * (y - y_0) * sin_c * cos_xi_0) / rho).asin();
        coord[1] = ellps.latitude_authalic_to_geographic(xi, &authalic);

        let num = (x - x_0) * sin_c;
        let denom = d * rho * cos_xi_0 * cos_c - d * d * (y - y_0) * sin_xi_0 * sin_c;
        coord[0] = num.atan2(denom) + lon_0;

        successes += 1;
    }

    Ok(successes)
}

// ----- C O N S T R U C T O R ---------------------------------------------------------

#[rustfmt::skip]
pub const GAMUT: [OpParameter; 6] = [
    OpParameter::Flag { key: "inv" },
    OpParameter::Text { key: "ellps", default: Some("GRS80") },

    OpParameter::Real { key: "lat_0", default: Some(0_f64) },
    OpParameter::Real { key: "lon_0", default: Some(0_f64) },

    OpParameter::Real { key: "x_0",   default: Some(0_f64) },
    OpParameter::Real { key: "y_0",   default: Some(0_f64) },
];

pub fn new(parameters: &RawParameters, _ctx: &dyn Context) -> Result<Op, Error> {
    let def = &parameters.definition;
    let mut params = ParsedParameters::new(parameters, &GAMUT)?;

    let lat_0 = params.lat[0];
    if lat_0.is_nan() {
        warn!("LAEA: Bad central latitude!");
        return Err(Error::BadParam("lat_0".to_string(), def.clone()));
    }

    let t = lat_0.abs();
    if t > FRAC_PI_2 + EPS10 {
        warn!("LAEA: Bad central latitude!");
        return Err(Error::BadParam("lat_0".to_string(), def.clone()));
    }

    let polar = (t - FRAC_PI_2).abs() < EPS10;
    let north = polar && (t > 0.0);
    let equatoreal = !polar && t < EPS10;
    let oblique = !polar && !equatoreal;
    match (polar, equatoreal, north) {
        (true, _, true) => params.boolean.insert("north_polar"),
        (true, _, false) => params.boolean.insert("south_polar"),
        (_, true, _) => params.boolean.insert("equatoreal"),
        _ => params.boolean.insert("oblique"),
    };

    // --- Precompute some latitude invariant factors ---

    let ellps = params.ellps[0];
    let a = ellps.semimajor_axis();
    let es = ellps.eccentricity_squared();
    let e = es.sqrt();
    let (sin_phi_0, cos_phi_0) = lat_0.sin_cos();

    // qs for the central parallel
    let q0 = qs(sin_phi_0, e);
    // qs for the North Pole
    let qp = qs(1.0, e);
    // Authalic latitude of the central parallel - 𝛽₀ in the IOGP text
    let xi_0 = (q0 / qp).asin();
    // Rq in the IOGP text
    let rq = a * (0.5 * qp).sqrt();
    // D in the IOGP text
    let d = if oblique {
        a * (cos_phi_0 / (1.0 - es * sin_phi_0 * sin_phi_0).sqrt()) / (rq * xi_0.cos())
    } else if equatoreal {
        rq.recip()
    } else {
        a
    };

    params.real.insert("xi_0", xi_0);
    params.real.insert("q0", q0);
    params.real.insert("qp", qp);
    params.real.insert("rq", rq);
    params.real.insert("d", d);

    let authalic = ellps.coefficients_for_authalic_latitude_computations();
    params.fourier_coefficients.insert("authalic", authalic);

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

// ----- A N C I L L A R Y   F U N C T I O N S -----------------------------------------

// ----- T E S T S ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laea_oblique() -> Result<(), Error> {
        let mut ctx = Minimal::default();

        // ETRS-LAEA grid definition
        let op = ctx.op("laea ellps=GRS80 lat_0=52 lon_0=10  x_0=4321000 y_0=3210000")?;

        // The test point from IOGP
        let p = Coord::geo(50.0, 5.0, 0.0, 0.0);
        let mut operands = [p];

        // Forward
        ctx.apply(op, Fwd, &mut operands)?;
        assert!((operands[0][0] - 3962799.45).abs() < 0.01);
        assert!((operands[0][1] - 2999718.85).abs() < 0.01);
        ctx.apply(op, Inv, &mut operands)?;
        assert!((operands[0][0].to_degrees() - 5.0).abs() < 1e-12);
        assert!((operands[0][1].to_degrees() - 50.).abs() < 1e-12);

        // Missing test points for the poar aspects

        Ok(())
    }
}
