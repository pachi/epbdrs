// Copyright (c) 2018-2019 Ministerio de Fomento
//               Instituto de Ciencias de la Construcción Eduardo Torroja (IETcc-CSIC)

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
//            Daniel Jiménez González <danielj@ietcc.csic.es>
//            Marta Sorribes Gil <msorribes@ietcc.csic.es>

#[macro_use]
extern crate clap;

use exitcode;

use serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;

use clap::{App, AppSettings, Arg};

use cteepbd::{cte, energy_performance, Balance, Components, MetaVec, RenNrenCo2, Service};

type Result<T, E = Box<dyn std::error::Error + Sync + Send>> = std::result::Result<T, E>;

// Funciones auxiliares -----------------------------------------------------------------------

fn readfile(path: &Path) -> Result<String> {
    let mut f = File::open(path)
        .map_err(|_e| format!("ERROR: archivo \"{}\" no encontrado", path.display()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .map_err(|_e| "ERROR: no se ha podido leer el archivo")?;
    Ok(contents)
}

fn writefile(path: &Path, content: &[u8]) {
    let mut file = File::create(&path)
        .map_err(|e| {
            panic!(
                "ERROR: no se ha podido escribir en \"{}\": {}",
                path.display(),
                e
            )
        })
        .unwrap();
    if let Err(e) = file.write_all(content) {
        panic!(
            "ERROR: no se ha podido escribir en \"{}\": {}",
            path.display(),
            e
        )
    }
}

// Funciones auxiliares de validación y obtención de valores

/// Comprueba validez del valor del factor de exportación de la CLI.
fn validate_kexp(matches: &clap::ArgMatches<'_>, verbosity: u64) {
    if matches.is_present("kexp") {
        let kexp = value_t!(matches, "kexp", f32).unwrap_or_else(|error| {
            eprintln!("ERROR: El área de referencia indicado no es un valor numérico válido");
            if verbosity > 2 {
                println!("{}", error)
            };
            exit(exitcode::DATAERR);
        });
        if kexp < 0.0 || kexp > 1.0 {
            eprintln!(
                "ERROR: el factor de exportación debe estar entre 0.00 y 1.00 y vale {:.2}",
                kexp
            );
            exit(exitcode::DATAERR);
        };
        if kexp != cte::KEXP_DEFAULT {
            println!(
                "AVISO: factor de exportación k_exp ({:.2}) distinto al reglamentario ({:.2})",
                kexp,
                cte::KEXP_DEFAULT
            );
        };
    }
}

/// Comprueba validez del dato de area en la CLI.
fn validate_arearef(matches: &clap::ArgMatches<'_>, verbosity: u64) {
    if matches.is_present("arearef") {
        let arearef = value_t!(matches, "arearef", f32).unwrap_or_else(|error| {
            println!("El área de referencia indicado no es un valor numérico válido");
            if verbosity > 2 {
                println!("{}", error)
            };
            exit(exitcode::DATAERR);
        });
        if arearef <= 1e-3 {
            eprintln!("ERROR: el área de referencia definida por el usuario debe ser mayor que 0.00 y vale {:.2}", arearef);
            exit(exitcode::DATAERR);
        }
    }
}

/// Obtiene factor de paso priorizando CLI -> metadatos -> None.
fn get_factor(
    matches_values: Option<clap::Values<'_>>,
    components: &mut Components,
    meta: &str,
    descr: &str,
    verbosity: u64,
) -> Option<RenNrenCo2> {
    // Origen del dato
    let mut orig = "";
    let factor = matches_values
        .and_then(|v| {
            let vv: Vec<f32> = v
                .map(|vv| {
                    f32::from_str(vv.trim()).unwrap_or_else(|_| {
                        eprintln!(
                            "ERROR: Formato incorrecto del factor de paso {:?}",
                            vv
                        );
                        exit(exitcode::DATAERR);
                    })
                })
                .collect();
            let userval = RenNrenCo2 {
                ren: vv[0],
                nren: vv[1],
                co2: vv[2],
            };
            // Dato desde línea de comandos
            orig = "usuario";
            Some(userval)
        })
        .or_else(|| {
            if let Some(metaval) = components.get_meta_rennren(meta) {
                orig = "metadatos de componentes";
                Some(metaval)
            } else {
                None
            }
        });
    if let Some(factor) = factor {
        if verbosity > 2 {
            println!("Factores de paso para {} ({}): {}", descr, orig, factor)
        };
        components.update_meta(
            meta,
            &format!("{:.3}, {:.3}, {:.3}", factor.ren, factor.nren, factor.co2),
        );
    };
    factor
}

/// Carga componentes desde archivo o devuelve componentes por defecto
fn get_components(archivo: Option<&str>) -> Components {
    if let Some(archivo_componentes) = archivo {
        let path = Path::new(archivo_componentes);
        let componentsstring = readfile(path).unwrap_or_else(|e| {
            eprintln!(
                "ERROR: No se ha podido leer el archivo de componentes energéticos {}: {}",
                path.display(),
                e
            );
            exit(exitcode::IOERR);
        });
        println!("Componentes energéticos: \"{}\"", path.display());
        cte::parse_components(&componentsstring).unwrap_or_else(|e| {
            eprintln!(
                "ERROR: Formato incorrecto del archivo de componentes \"{}\" ({})",
                archivo_componentes,
                e
            );
            exit(exitcode::DATAERR);
        })
    } else {
        Default::default()
    }
}

/// Obtén área de referencia, arearef
/// Argumentos de CLI > Metadatos de componentes > Valor por defecto (AREAREF_DEFAULT = 1.0)
fn get_arearef(components: &Components, matches: &clap::ArgMatches<'_>) -> f32 {
    let mut arearef;
    // Se define CTE_AREAREF en metadatos de componentes energéticos
    if components.has_meta("CTE_AREAREF") {
        arearef = components.get_meta_f32("CTE_AREAREF").unwrap_or_else(|| {
            println!("El área de referencia de los metadatos no es un valor numérico válido");
            exit(exitcode::DATAERR);
        });
        if matches.occurrences_of("arearef") == 0 {
            println!("Área de referencia (metadatos) [m2]: {:.2}", arearef);
        } else {
            let m_arearef = value_t!(matches, "arearef", f32).unwrap();
            if (arearef - m_arearef).abs() > 1e-3 {
                println!("AVISO: El valor del área de referencia del archivo de componentes energéticos ({:.2}) no coincide con el valor definido por el usuario ({:.2})", arearef, m_arearef);
            }
            arearef = m_arearef;
            println!("Área de referencia (usuario) [m2]: {:.2}", arearef);
        }
    // Área de referencia en la interfaz
    } else if matches.occurrences_of("arearef") != 0 {
        arearef = value_t!(matches, "arearef", f32).unwrap();
        println!("Área de referencia (usuario) [m2]: {:.2}", arearef);
    // Valor por defecto
    } else {
        arearef = cte::AREAREF_DEFAULT;
        println!("Área de referencia (predefinida) [m2]: {:.2}", arearef);
    }
    arearef
}

/// Obtén factor de exportación, kexp
/// Argumentos de CLI > Metadatos de componentes > Valor por defecto (KEXP_REF = 0.0)
fn get_kexp(components: &Components, matches: &clap::ArgMatches<'_>) -> f32 {
    let mut kexp;
    // Se define CTE_KEXP en metadatos de componentes energéticos
    if components.has_meta("CTE_KEXP") {
        kexp = components.get_meta_f32("CTE_KEXP").unwrap_or_else(|| {
            println!("El factor de exportación de los metadatos no es un valor numérico válido");
            exit(exitcode::DATAERR);
        });
        if matches.occurrences_of("kexp") == 0 {
            println!("Factor de exportación (metadatos) [-]: {:.1}", kexp);
        } else {
            let m_kexp = value_t!(matches, "kexp", f32).unwrap();
            if (kexp - m_kexp).abs() > 1e-3 {
                println!("AVISO: El valor del factor de exportación del archivo de componentes energéticos ({:.1}) no coincide con el valor definido por el usuario ({:.1})", kexp, m_kexp);
            }
            kexp = m_kexp;
            println!("Factor de exportación (usuario) [-]: {:.1}", kexp);
        }
    // kexp definido en la interfaz
    } else if matches.occurrences_of("kexp") != 0 {
        kexp = value_t!(matches, "kexp", f32).unwrap();
        println!("Factor de exportación (usuario) [-]: {:.1}", kexp);
    // Valor por defecto
    } else {
        kexp = cte::KEXP_DEFAULT;
        println!("Factor de exportación (predefinido) [-]: {:.1}", kexp);
    }
    kexp
}

// Función principal ------------------------------------------------------------------------------

fn main() {
    let matches = App::new("CteEPBD")
        .bin_name("cteepbd")
        .version(env!("CARGO_PKG_VERSION"))
        .author("
Copyright (c) 2018-2019 Ministerio de Fomento,
              Instituto de CC. de la Construcción Eduardo Torroja (IETcc-CSIC)

Autores: Rafael Villar Burke <pachi@ietcc.csic.es>,
         Daniel Jiménez González <danielj@ietcc.csic.es>
         Marta Sorribes Gil <msorribes@ietcc.csic.es>

Licencia: Publicado bajo licencia MIT.

")
        .about("CteEpbd - Eficiencia energética de los edificios (CTE DB-HE).")
        .setting(AppSettings::NextLineHelp)
        .arg(Arg::with_name("arearef")
            .short("a")
            .long("arearef")
            .value_name("AREAREF")
            .default_value("1.0")
            .help("Área de referencia")
            .takes_value(true)
            .display_order(1))
        .arg(Arg::with_name("kexp")
            .short("k")
            .long("kexp")
            .default_value("0.0")
            .value_name("KEXP")
            .help("Factor de exportación (k_exp)")
            .takes_value(true)
            .display_order(2))
        .arg(Arg::with_name("archivo_componentes")
            .short("c")
            .long("archivo_componentes")
            .value_name("ARCHIVO_COMPONENTES")
            .help("Archivo de definición de los componentes energéticos")
            .takes_value(true)
            //.validator(clap_validators::fs::is_file))
            .display_order(4))
        .arg(Arg::with_name("archivo_factores")
            .short("f")
            .long("archivo_factores")
            .value_name("ARCHIVO_FACTORES")
            .required_unless_one(&["fps_loc", "archivo_componentes"])
            .conflicts_with_all(&["fps_loc", "cogen", "red1", "red2"])
            .help("Archivo de definición de los componentes energéticos")
            .takes_value(true)
            //.validator(clap_validators::fs::is_file))
            .display_order(5))
        .arg(Arg::with_name("fps_loc")
            .short("l")
            .value_name("LOCALIZACION")
            .possible_values(&["PENINSULA", "CANARIAS", "BALEARES", "CEUTAMELILLA"])
            .required_unless_one(&["archivo_factores", "archivo_componentes"])
            .help("Localización que define los factores de paso\n")
            .takes_value(true)
            .display_order(6))
        // Archivos de salida
        .arg(Arg::with_name("gen_archivo_componentes")
            .long("oc")
            .value_name("GEN_ARCHIVO_COMPONENTES")
            .help("Archivo de salida de los vectores energéticos corregidos")
            .takes_value(true))
        .arg(Arg::with_name("gen_archivo_factores")
            .long("of")
            .value_name("GEN_ARCHIVO_FACTORES")
            .help("Archivo de salida de los factores de paso corregidos")
            .takes_value(true))
        .arg(Arg::with_name("archivo_salida_json")
            .long("json")
            .value_name("ARCHIVO_SALIDA_JSON")
            .help("Archivo de salida de resultados detallados en formato JSON")
            .takes_value(true))
        .arg(Arg::with_name("archivo_salida_xml")
            .long("xml")
            .value_name("ARCHIVO_SALIDA_XML")
            .help("Archivo de salida de resultados detallados en formato XML")
            .takes_value(true))
        .arg(Arg::with_name("archivo_salida_txt")
            .long("txt")
            .value_name("ARCHIVO_SALIDA_TXT")
            .help("Archivo de salida de resultados detallados en formato texto simple")
            .takes_value(true))
        // Factores definidos por el usuario
        .arg(Arg::with_name("cogen")
            .long("cogen")
            .value_names(&["COGEN_ren", "COGEN_nren", "COGEN_co2"])
            .help("Factores de exportación a red (ren, nren, co2) de electricidad cogenerada.\nP.e.: --cogen 0 2.5 0.3")
            .takes_value(true)
            .number_of_values(3))
        .arg(Arg::with_name("red1")
            .long("red1")
            .value_names(&["RED1_ren", "RED1_nren", "RED1_co2"])
            .help("Factores de paso (ren, nren, co2) de la producción del vector RED1.\nP.e.: --red1 0 1.3 0.3")
            .takes_value(true)
            .number_of_values(3))
        .arg(Arg::with_name("red2")
            .long("red2")
            .value_names(&["RED2_ren", "RED2_nren", "RED2_co2"])
            .help("Factores de paso (ren, nren, co2) de la producción del vector RED2.\nP.e.: --red2 0 1.3 0.3")
            .takes_value(true)
            .number_of_values(2))
        // Cálculo para servicio de ACS y factores en perímetro nearby
        .arg(Arg::with_name("acsnrb")
            .short("N")
            .long("acs_nearby")
            .requires("archivo_componentes")
            .help("Realiza el balance considerando solo el servicio de ACS y el perímetro nearby"))
        // Simplificación de factores
        .arg(Arg::with_name("nosimplificafps")
            .short("F")
            .long("no_simplifica_fps")
            .help("Evita la simplificación de los factores de paso según los vectores definidos"))
        // Opciones estándar: licencia y nivel de detalle
        .arg(Arg::with_name("showlicense")
            .short("L")
            .long("licencia")
            .help("Muestra la licencia del programa (MIT)"))
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .get_matches();

    if matches.is_present("showlicense") {
        println!(
            "
Copyright (c) 2018-2019 Ministerio de Fomento
              Instituto de Ciencias de la Construcción Eduardo Torroja (IETcc-CSIC)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the 'Software'), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED 'AS IS', WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Author(s): Rafael Villar Burke <pachi@ietcc.csic.es>
            Daniel Jiménez González <danielj@ietcc.csic.es>
            Marta Sorribes Gil <msorribes@ietcc.csic.es>"
        );
        exit(exitcode::OK);
    }

    // Prólogo ------------------------------------------------------------------------------------

    let verbosity = matches.occurrences_of("v");

    if verbosity > 2 {
        println!("Opciones indicadas: ----------");
        println!("{:#?}", matches);
        println!("------------------------------");
    }

    println!("** Datos de entrada");

    // Componentes energéticos ---------------------------------------------------------------------
    let mut components = get_components(matches.value_of("archivo_componentes"));

    // Cálculo para servicio de ACS en nearby
    if matches.is_present("acsnrb") {
        components = cte::components_by_service(&components, Service::ACS)
    }

    if verbosity > 1 && !components.cmeta.is_empty() {
        println!("Metadatos de componentes:");
        for meta in &components.cmeta {
            println!("  {}: {}", meta.key, meta.value);
        }
    }

    // Comprobación del parámetro de factor de exportación kexp ----------------------------------------
    validate_kexp(&matches, verbosity);

    // Comprobación del parámetro de área de referencia -------------------------------------------------------------------------
    validate_arearef(&matches, verbosity);

    // Factores de paso ---------------------------------------------------------------------------

    // 0. Factores por defecto, según modo
    let default_wf = cte::WF_RITE2014;

    // 1. Factores de paso definibles por el usuario (a través de la CLI o de metadatos)
    let user_wf = cte::CteUserWF {
        red1: get_factor(
            matches.values_of("red1"),
            &mut components,
            "CTE_RED1",
            "RED1",
            verbosity,
        ),
        red2: get_factor(
            matches.values_of("red2"),
            &mut components,
            "CTE_RED2",
            "RED2",
            verbosity,
        ),
        cogen_to_grid: get_factor(
            matches.values_of("cogen"),
            &mut components,
            "CTE_COGEN",
            "COGENERACION a la red",
            verbosity,
        ),
        cogen_to_nepb: get_factor(
            matches.values_of("cogennepb"),
            &mut components,
            "CTE_COGENNEPB",
            "COGENERACION a usos no EPB",
            verbosity,
        ),
    };

    // 2. Definición de los factores de paso principales
    let mut fpdata =
        // Definición desde archivo
        if let Some(archivo_factores) = matches.value_of("archivo_factores") {
            let path = Path::new(archivo_factores);
            let fpstring = readfile(path)
                .and_then(|fpstring| {
                    println!("Factores de paso (archivo): \"{}\"", path.display());
                    Ok(fpstring)
                })
                .unwrap_or_else(|e| {
                    eprintln!(
                        "ERROR: No se ha podido leer el archivo de factores de paso \"{}\" -> {}",
                        path.display(), e
                    );
                    exit(exitcode::IOERR);
                });
            cte::wfactors_from_str(&fpstring, &user_wf, &default_wf)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "ERROR: No se ha podido interpretar el archivo de factores de paso \"{}\" -> {}",
                        path.display(), e
                    );
                    exit(exitcode::DATAERR);
                })
        // Definición por localización
        } else {
            let localizacion = matches
                // 1/2 Desde opción de CLI
                .value_of("fps_loc")
                .and_then(|v| {
                    println!("Factores de paso (usuario): {}", v);
                    components.update_meta("CTE_LOCALIZACION", v);
                    Some(v.to_string())
                })
                // 2/2 desde metadatos de componentes
                .or_else(|| components.get_meta("CTE_LOCALIZACION")
                    .and_then(|loc| {
                        println!("Factores de paso (metadatos): {}", loc);
                        Some(loc)
                    })
                )
                // Error
                .or_else(|| {
                    eprintln!("ERROR: Sin datos suficientes para determinar los factores de paso");
                    exit(exitcode::USAGE);
                }).unwrap();
            cte::wfactors_from_loc(&localizacion, &user_wf, &default_wf)
                .unwrap_or_else(|e| {
                    println!("ERROR: No se han podido generar los factores de paso: {}", e);
                    exit(exitcode::DATAERR);
                })
        };

    // Simplificación de los factores de paso -----------------------------------------------------------------
    if !matches.is_present("nosimplificafps") && !components.cdata.is_empty() {
        let oldfplen = fpdata.wdata.len();
        cte::strip_wfactors(&mut fpdata, &components);
        if verbosity > 1 {
            println!(
                "Reducción de factores de paso: {} a {}",
                oldfplen,
                fpdata.wdata.len()
            );
        }
    }

    // Transformación a factores de paso en nearby
    if matches.is_present("acsnrb") {
        // Estamos en cálculo de ACS en nearby
        fpdata = cte::wfactors_to_nearby(&fpdata);
    }

    // Área de referencia -------------------------------------------------------------------------
    // Argumentos de CLI > Metadatos de componentes > Valor por defecto (AREA_REF = 1)
    let arearef = get_arearef(&components, &matches);

    // Actualiza metadato CTE_AREAREF al valor seleccionado
    components.update_meta("CTE_AREAREF", &format!("{:.2}", arearef));

    // kexp ------------------------------------------------------------------------------------------
    // Argumentos de CLI > Metadatos de componentes > Valor por defecto (KEXP_REF = 0.0)
    let kexp = get_kexp(&components, &matches);

    // Actualiza metadato CTE_KEXP al valor seleccionado
    components.update_meta("CTE_KEXP", &format!("{:.1}", kexp));

    // Guardado de componentes energéticos -----------------------------------------------------------
    if matches.is_present("gen_archivo_componentes") {
        let path = Path::new(matches.value_of_os("gen_archivo_componentes").unwrap());
        let components_string = format!("{}", components);
        if verbosity > 2 {
            println!("Componentes energéticos:\n{}", components_string);
        }
        writefile(&path, components_string.as_bytes());
        if verbosity > 0 {
            println!(
                "Guardado archivo de componentes energéticos: {}",
                path.display()
            );
        }
    }

    // Guardado de factores de paso corregidos ------------------------------------------------------
    if matches.is_present("gen_archivo_factores") {
        let path = Path::new(matches.value_of_os("gen_archivo_factores").unwrap());
        let fpstring = format!("{}", fpdata);
        if verbosity > 2 {
            println!("Factores de paso:\n{}", fpstring);
        }
        writefile(&path, fpstring.as_bytes());
        if verbosity > 0 {
            println!("Guardado archivo de factores de paso: {}", path.display());
        }
    }

    // Cálculo del balance -------------------------------------------------------------------------
    let balance: Option<Balance> = if !components.cdata.is_empty() {
        Some(
            energy_performance(&components, &fpdata, kexp, arearef).unwrap_or_else(|e| {
                eprintln!("ERROR: No se ha podido calcular el balance energético: {}", e);
                exit(exitcode::DATAERR);
            }),
        )
    } else if matches.is_present("gen_archivos_factores") {
        println!(
            "No se calcula el balance pero se ha generado el archivo de factores de paso {}",
            matches.value_of("gen_archivo_factores").unwrap()
        );
        None
    } else {
        println!("No se han definido datos suficientes para calcular el balance energético. Necesita definir al menos los componentes energéticos y los factores de paso");
        None
    };

    // Salida de resultados ------------------------------------------------------------------------
    if let Some(balance) = balance {
        // Guardar balance en formato json
        if matches.is_present("archivo_salida_json") {
            let path = Path::new(matches.value_of_os("archivo_salida_json").unwrap());
            if verbosity > 0 {
                println!("Resultados en formato JSON: {:?}", path.display());
            }
            let json = serde_json::to_string_pretty(&balance).unwrap_or_else(|error| {
                eprintln!("ERROR: No se ha podido convertir el balance al formato JSON");
                if verbosity > 2 {
                    println!("{}", error)
                };
                exit(exitcode::DATAERR);
            });
            writefile(&path, json.as_bytes());
        }
        // Guardar balance en formato XML
        if matches.is_present("archivo_salida_xml") {
            let path = Path::new(matches.value_of_os("archivo_salida_xml").unwrap());
            if verbosity > 0 {
                println!("Resultados en formato XML: {:?}", path.display());
            }
            let xml = cte::balance_to_xml(&balance);
            writefile(&path, xml.as_bytes());
        }
        // Mostrar siempre en formato de texto plano
        if matches.is_present("acsnrb") {
            println!("** Balance energético (servicio de ACS, perímetro próximo)");
        } else {
            println!("** Balance energético");
        }
        let plain = cte::balance_to_plain(&balance);
        println!("{}", plain);

        // Guardar balance en formato de texto plano
        if matches.is_present("archivo_salida_txt") {
            let path = Path::new(matches.value_of_os("archivo_salida_txt").unwrap());
            if verbosity > 0 {
                println!("Resultados en formato XML: {:?}", path.display());
            }
            writefile(&path, plain.as_bytes());
        }
    };
}
