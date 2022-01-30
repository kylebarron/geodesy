use super::internal::*;

#[derive(Debug)]
pub struct ParsedParameters {
    pub name: String,

    // Commonly used options have hard-coded slots
    ellps: [Ellipsoid; 2],
    lat: [f64; 4],
    lon: [f64; 4],
    x: [f64; 4],
    y: [f64; 4],
    k: [f64; 4],

    // Op-specific options are stored in B-Trees
    boolean: BTreeSet<&'static str>,
    natural: BTreeMap<&'static str, usize>,
    integer: BTreeMap<&'static str, i64>,
    real: BTreeMap<&'static str, f64>,
    series: BTreeMap<&'static str, Vec<f64>>,
    text: BTreeMap<&'static str, String>,
    uuid: BTreeMap<&'static str, uuid::Uuid>,
    ignored: Vec<String>,
}

// Accessors
impl ParsedParameters {
    pub fn boolean(&self, key: &str) -> bool {
        self.boolean.contains(key)
    }
    pub fn natural(&self, key: &str) -> Result<usize, Error> {
        if let Some(value) = self.natural.get(key) {
            return Ok(*value);
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn integer(&self, key: &str) -> Result<i64, Error> {
        if let Some(value) = self.integer.get(key) {
            return Ok(*value);
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn real(&self, key: &str) -> Result<f64, Error> {
        if let Some(value) = self.real.get(key) {
            return Ok(*value);
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn series(&self, key: &str) -> Result<&[f64], Error> {
        if let Some(value) = self.series.get(key) {
            return Ok(value);
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn text(&self, key: &str) -> Result<String, Error> {
        if let Some(value) = self.text.get(key) {
            return Ok(value.to_string());
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn uuid(&self, key: &str) -> Result<uuid::Uuid, Error> {
        if let Some(value) = self.uuid.get(key) {
            return Ok(*value);
        }
        Err(Error::MissingParam(key.to_string()))
    }
    pub fn ignored(&self) -> Vec<String> {
        self.ignored.clone()
    }
    pub fn ellps(&self, index: usize) -> &Ellipsoid {
        &self.ellps[index]
    }
    pub fn x(&self, index: usize) -> f64 {
        self.x[index]
    }
    pub fn y(&self, index: usize) -> f64 {
        self.y[index]
    }
    pub fn lat(&self, index: usize) -> f64 {
        self.lat[index]
    }
    pub fn lon(&self, index: usize) -> f64 {
        self.lon[index]
    }
    pub fn k(&self, index: usize) -> f64 {
        self.k[index]
    }
}

impl ParsedParameters {
    pub fn new(
        parameters: &RawParameters,
        gamut: &[OpParameter],
    ) -> Result<ParsedParameters, Error> {
        let locals = etc::split_into_parameters(&parameters.definition);
        let globals = &parameters.globals;
        let mut boolean = BTreeSet::<&'static str>::new();
        let mut natural = BTreeMap::<&'static str, usize>::new();
        let mut integer = BTreeMap::<&'static str, i64>::new();
        let mut real = BTreeMap::<&'static str, f64>::new();
        let mut series = BTreeMap::<&'static str, Vec<f64>>::new();
        let mut text = BTreeMap::<&'static str, String>::new();
        #[allow(unused_mut)]
        let mut uuid = BTreeMap::<&'static str, uuid::Uuid>::new();

        // Try to locate all accepted parameters, type check, and place them into
        // their proper bins
        for p in gamut {
            match *p {
                OpParameter::Flag { key } => {
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        if value.is_empty() || value.to_lowercase() == "true" {
                            boolean.insert(key);
                            continue;
                        }
                        warn!("Cannot parse {key}:{value} as a boolean constant!");
                        return Err(Error::BadParam(key.to_string(), value));
                    }
                    // If we're here, the key was not found, and we're done, since
                    // flags are always optional (i.e. implicitly false when not given)
                    continue;
                }

                OpParameter::Natural { key, default } => {
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        if let Ok(v) = value.parse::<usize>() {
                            natural.insert(key, v);
                            continue;
                        }
                        warn!("Cannot parse {key}:{value} as a natural number!");
                        return Err(Error::BadParam(key.to_string(), value));
                    }

                    // Key not found - default given?
                    if let Some(value) = default {
                        natural.insert(key, value);
                        continue;
                    }

                    error!("Missing required parameter '{key}'");
                    return Err(Error::MissingParam(key.to_string()));
                }

                OpParameter::Integer { key, default } => {
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        if let Ok(v) = value.parse::<i64>() {
                            integer.insert(key, v);
                            continue;
                        }
                        warn!("Cannot parse {key}:{value} as an integer!");
                        return Err(Error::BadParam(key.to_string(), value));
                    }

                    // If we're here, the key was not found

                    // Default given?
                    if let Some(value) = default {
                        integer.insert(key, value);
                        continue;
                    }

                    // Missing a required parameter
                    error!("Missing required parameter '{key}'");
                    return Err(Error::MissingParam(key.to_string()));
                }

                OpParameter::Real { key, default } => {
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        if let Ok(v) = value.parse::<f64>() {
                            real.insert(key, v);
                            continue;
                        }
                        warn!("Cannot parse {key}:{value} as a real number");
                        return Err(Error::BadParam(key.to_string(), value));
                    }

                    // If we're here, the key was not found

                    // Default given?
                    if let Some(value) = default {
                        real.insert(key, value);
                        continue;
                    }

                    // Missing a required parameter
                    error!("Missing required parameter '{key}'");
                    return Err(Error::MissingParam(key.to_string()));
                }

                // TODO! (only reads first element of the series, and puts it into the Real store)
                OpParameter::Series { key, default } => {
                    let mut elements = Vec::<f64>::new();
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        for element in value.split(',') {
                            if let Ok(v) = element.parse::<f64>() {
                                elements.push(v);
                                continue;
                            }
                            warn!("Cannot parse {key}:{value} as a series");
                            return Err(Error::BadParam(key.to_string(), value.to_string()));
                        }
                        series.insert(key, elements);
                        continue;
                    }

                    // If we're here, the key was not found

                    // Default given?
                    if let Some(value) = default {
                        // Defaults to nothing, so we just continue with the next parameter
                        if value.is_empty() {
                            continue;
                        }
                        for element in value.split(',') {
                            if let Ok(v) = element.parse::<f64>() {
                                elements.push(v);
                                continue;
                            }
                            warn!("Cannot parse {key}:{value} as a series");
                            return Err(Error::BadParam(key.to_string(), value.to_string()));
                        }
                        series.insert(key, elements);
                        continue;
                    }

                    // Missing a required parameter
                    error!("Missing required parameter '{key}'");
                    return Err(Error::MissingParam(key.to_string()));
                }

                OpParameter::Text { key, default } => {
                    if let Some(value) = etc::chase(globals, &locals, key)? {
                        // should chase!
                        text.insert(key, value.to_string());
                        continue;
                    }

                    // If we're here, the key was not found

                    // Default given?
                    if let Some(value) = default {
                        text.insert(key, value.to_string());
                        continue;
                    }

                    // Missing a required parameter
                    error!("Missing required parameter '{key}'");
                    return Err(Error::MissingParam(key.to_string()));
                }
            };
        }

        let ellps = [Ellipsoid::default(), Ellipsoid::default()];
        let lat = [0.; 4];
        let lon = [0.; 4];
        let x = [0.; 4];
        let y = [0.; 4];
        let k = [0.; 4];

        let name = locals
            .get("name")
            .unwrap_or(&"unknown".to_string())
            .to_string();

        // Params explicitly set to the default value
        // let mut redundant = BTreeSet::<String>::new();
        // Params specified, but not used
        let ignored: Vec<String> = locals.into_keys().collect();
        Ok(ParsedParameters {
            ellps,
            lat,
            lon,
            x,
            y,
            k,
            name,
            boolean,
            natural,
            integer,
            real,
            series,
            text,
            uuid,
            ignored,
        })
    }
}

// ----- T E S T S ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    const GAMUT: [OpParameter; 6] = [
        OpParameter::Flag    { key: "flag" },
        OpParameter::Natural { key: "natural",  default: Some(0) },
        OpParameter::Integer { key: "integer",  default: Some(-1)},
        OpParameter::Real    { key: "real",     default: Some(1.25) },
        OpParameter::Series  { key: "series",   default: Some("1,2,3,4") },
        OpParameter::Text    { key: "text",     default: Some("text") },
    ];

    #[test]
    fn basic() -> Result<(), Error> {
        let invocation = String::from("cucumber flag");
        let globals = BTreeMap::<String, String>::new();
        let raw = RawParameters::new(&invocation, &globals);
        let p = ParsedParameters::new(&raw, &GAMUT)?;
        // println!("{:#?}", p);

        // Booleans correctly parsed?
        assert!(
            p.boolean.get("flag").is_some(),
            "`flag` not in registered booleans: {:#?}",
            p.boolean
        );
        assert!(
            p.boolean.get("galf").is_none(),
            "`galf` not in registered booleans: {:?}",
            p.boolean
        );

        // Series correctly parsed?
        let series = p.series.get("series").unwrap();
        assert_eq!(series.len(), 4);
        assert_eq!(series[0], 1.);
        assert_eq!(series[3], 4.);

        // Etc.
        assert_eq!(*p.natural.get("natural").unwrap(), 0_usize);
        assert_eq!(*p.integer.get("integer").unwrap(), -1);
        assert_eq!(*p.text.get("text").unwrap(), "text");

        Ok(())
    }
}