// Java shim handlers - to be inserted into lib.rs

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::{CfmlError, CfmlResult};
use indexmap::IndexMap;

pub fn handle_java_messagedigest(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" | "getinstance" => {
            let algorithm = args
                .first()
                .map(|a| a.as_string().to_lowercase())
                .unwrap_or_else(|| "sha-256".to_string());
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.security.messagedigest".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__algorithm".to_string(), CfmlValue::string(algorithm));
            shim.insert("__data".to_string(), CfmlValue::string(String::new()));
            Ok(CfmlValue::strukt(shim))
        }
        "update" => {
            // Real Java MessageDigest.update takes a byte[]. We accept both
            // Binary (from "...".getBytes()) and String (lenient) so Lucee and
            // RustCFML run the same interop code without rewrites.
            if let CfmlValue::Struct(ref shim) = object {
                let current = shim
                    .get("__data")
                    .map(|d| d.as_string())
                    .unwrap_or_default();
                let input = match args.first() {
                    Some(CfmlValue::Binary(b)) => String::from_utf8_lossy(b).to_string(),
                    Some(v) => v.as_string(),
                    None => String::new(),
                };
                let mut new_shim = shim.snapshot();
                new_shim.insert(
                    "__data".to_string(),
                    CfmlValue::string(format!("{}{}", current, input)),
                );
                Ok(CfmlValue::strukt(new_shim))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "digest" => {
            if let CfmlValue::Struct(ref shim) = object {
                let data = shim
                    .get("__data")
                    .map(|d| d.as_string())
                    .unwrap_or_default();
                Ok(CfmlValue::Binary(data.into_bytes()))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "isequal" => {
            if args.len() >= 2 {
                Ok(CfmlValue::Bool(args[0].as_string() == args[1].as_string()))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "reset" => {
            if let CfmlValue::Struct(ref shim) = object {
                let mut new_shim = shim.snapshot();
                new_shim.insert("__data".to_string(), CfmlValue::string(String::new()));
                Ok(CfmlValue::strukt(new_shim))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_uuid(method: &str, _args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    match method {
        "init" | "randomuuid" => {
            let uuid = format!("{:032x}", rand_u128());
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.uuid".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__uuid".to_string(), CfmlValue::string(uuid));
            Ok(CfmlValue::strukt(shim))
        }
        "tostring" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(uuid)) = shim.get("__uuid") {
                    if uuid.len() >= 32 {
                        let formatted = format!(
                            "{}-{}-{}-{}-{}",
                            &uuid[0..8],
                            &uuid[8..12],
                            &uuid[12..16],
                            &uuid[16..20],
                            &uuid[20..32]
                        );
                        return Ok(CfmlValue::string(formatted));
                    }
                }
            }
            Ok(CfmlValue::Null)
        }
        "getversion" => Ok(CfmlValue::Int(4)),
        "getvariant" => Ok(CfmlValue::Int(2)),
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_thread(method: &str, _args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    // "threadgroup" is a nested shim for java.lang.ThreadGroup accessed via
    // Thread.getThreadGroup(). We route its own methods here too.
    if let CfmlValue::Struct(ref shim) = object {
        if shim
            .get("__java_class")
            .map(|v| v.as_string())
            .unwrap_or_default()
            == "java.lang.threadgroup"
        {
            return match method {
                "getname" => Ok(shim
                    .get("__name")
                    .unwrap_or(CfmlValue::string("main".to_string()))),
                _ => Ok(CfmlValue::Null),
            };
        }
    }
    match method {
        "init" | "currentthread" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.lang.thread".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__name".to_string(), CfmlValue::string("main".to_string()));
            Ok(CfmlValue::strukt(shim))
        }
        "getname" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(shim
                    .get("__name")
                    .unwrap_or(CfmlValue::string("main".to_string())))
            } else {
                Ok(CfmlValue::string("main".to_string()))
            }
        }
        "getthreadgroup" => {
            let mut tg = IndexMap::new();
            tg.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.lang.threadgroup".to_string()),
            );
            tg.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            tg.insert("__name".to_string(), CfmlValue::string("main".to_string()));
            Ok(CfmlValue::strukt(tg))
        }
        "getpriority" => Ok(CfmlValue::Int(5)),
        "isdaemon" => Ok(CfmlValue::Bool(false)),
        "sleep" => Ok(CfmlValue::Null),
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_inetaddress(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "getlocalhost" => {
            let hostname = std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("HOST"))
                .unwrap_or_else(|_| "localhost".to_string());
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.net.inetaddress".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert(
                "__hostname".to_string(),
                CfmlValue::string(hostname.clone()),
            );
            shim.insert(
                "__address".to_string(),
                CfmlValue::string("127.0.0.1".to_string()),
            );
            Ok(CfmlValue::strukt(shim))
        }
        "getbyname" => {
            let hostname = args
                .first()
                .map(|a| a.as_string())
                .unwrap_or_else(|| "localhost".to_string());
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.net.inetaddress".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert(
                "__hostname".to_string(),
                CfmlValue::string(hostname.clone()),
            );
            shim.insert(
                "__address".to_string(),
                CfmlValue::string("127.0.0.1".to_string()),
            );
            Ok(CfmlValue::strukt(shim))
        }
        "gethostname" | "gethostaddress" | "getcanonicalhostname" | "tostring" => {
            if let CfmlValue::Struct(ref shim) = object {
                let key = match method {
                    "gethostname" | "tostring" => "__hostname",
                    "gethostaddress" => "__address",
                    _ => "__hostname",
                };
                Ok(shim
                    .get(key)
                    .unwrap_or(CfmlValue::string("localhost".to_string())))
            } else {
                Ok(CfmlValue::string("localhost".to_string()))
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_file(method: &str, args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    match method {
        "init" => {
            let path = args.first().map(|a| a.as_string()).unwrap_or_default();
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.io.file".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__path".to_string(), CfmlValue::string(path));
            Ok(CfmlValue::strukt(shim))
        }
        "tostring" => {
            // java.io.File.toString() returns the original path as given.
            if let CfmlValue::Struct(ref shim) = object {
                return Ok(shim
                    .get("__path")
                    .unwrap_or(CfmlValue::string(String::new())));
            }
            Ok(CfmlValue::string(String::new()))
        }
        "getabsolute_path" | "getabsolutepath" | "getcanonicalpath" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    let p = std::path::Path::new(path.as_str());
                    if p.is_absolute() {
                        return Ok(CfmlValue::string(path.to_string()));
                    }
                    if let Ok(cwd) = std::env::current_dir() {
                        return Ok(CfmlValue::string(
                            cwd.join(path.as_str()).to_string_lossy().to_string(),
                        ));
                    }
                }
            }
            Ok(CfmlValue::string(String::new()))
        }
        "isabsolute" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    return Ok(CfmlValue::Bool(std::path::Path::new(path.as_str()).is_absolute()));
                }
            }
            Ok(CfmlValue::Bool(false))
        }
        "exists" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    return Ok(CfmlValue::Bool(std::path::Path::new(path.as_str()).exists()));
                }
            }
            Ok(CfmlValue::Bool(false))
        }
        "isfile" | "is_directory" | "isdirectory" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    let p = std::path::Path::new(path.as_str());
                    return Ok(CfmlValue::Bool(if method == "isfile" {
                        p.is_file()
                    } else {
                        p.is_dir()
                    }));
                }
            }
            Ok(CfmlValue::Bool(false))
        }
        "getname" | "lastmodified" | "length" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    if let Ok(meta) = std::fs::metadata(path.as_str()) {
                        if method == "getname" {
                            if let Some(n) = std::path::Path::new(path.as_str()).file_name() {
                                return Ok(CfmlValue::string(n.to_string_lossy().to_string()));
                            }
                        } else if method == "lastmodified" {
                            if let Ok(t) = meta.modified() {
                                let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                                return Ok(CfmlValue::Double(d.as_millis() as f64));
                            }
                        } else {
                            return Ok(CfmlValue::Int(meta.len() as i64));
                        }
                    }
                }
            }
            Ok(CfmlValue::Int(0))
        }
        "topath" => {
            // File.toPath() returns a java.nio.file.Path. This is the portable
            // alternative to Paths.get(…), which Lucee can't dispatch to
            // cleanly due to its String/varargs signature.
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(path) = shim.get("__path") {
                    let mut ps = IndexMap::new();
                    ps.insert(
                        "__java_class".to_string(),
                        CfmlValue::string("java.nio.file.paths".to_string()),
                    );
                    ps.insert("__java_shim".to_string(), CfmlValue::Bool(true));
                    ps.insert("__path".to_string(), path.clone());
                    return Ok(CfmlValue::strukt(ps));
                }
            }
            Ok(CfmlValue::Null)
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_system(method: &str, args: Vec<CfmlValue>, _object: &CfmlValue) -> CfmlResult {
    match method {
        "init" => {
            // java.lang.System is a static-only class in real Java, but we
            // return a shim struct so both init() and static-style access work.
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.lang.system".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            // Expose `out` as a nested shim so `system.out.println(...)` works.
            let mut out = IndexMap::new();
            out.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.lang.system.out".to_string()),
            );
            out.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("out".to_string(), CfmlValue::strukt(out));
            Ok(CfmlValue::strukt(shim))
        }
        "currenttimemillis" => {
            Ok(CfmlValue::Double(cfml_common::clock::now_unix_millis() as f64))
        }
        "nanotime" => {
            Ok(CfmlValue::Double(cfml_common::clock::now_unix_nanos() as f64))
        }
        "getproperty" => {
            // Some callers pass the key as the first "real" arg, but member
            // dispatch prepends the object — skip leading shim structs.
            let key = args
                .iter()
                .find_map(|a| match a {
                    CfmlValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let val = match key.to_lowercase().as_str() {
                "os.name" => std::env::consts::OS.to_string(),
                "file.separator" => std::path::MAIN_SEPARATOR.to_string(),
                "path.separator" => {
                    if cfg!(unix) {
                        ":".to_string()
                    } else {
                        ";".to_string()
                    }
                }
                "user.dir" => std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "user.home" => std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_default(),
                "java.version" => "rustcfml".to_string(),
                _ => String::new(),
            };
            Ok(CfmlValue::string(val))
        }
        "getenv" => {
            // No-arg form returns a struct of all env vars (real Java returns a Map).
            // Single-arg form returns the value for that key.
            let key = args.iter().find_map(|a| match a {
                CfmlValue::String(s) => Some(s.clone()),
                _ => None,
            });
            match key {
                Some(k) => Ok(CfmlValue::string(std::env::var(k.as_str()).unwrap_or_default())),
                None => {
                    let mut env = IndexMap::new();
                    for (k, v) in std::env::vars() {
                        env.insert(k, CfmlValue::string(v));
                    }
                    Ok(CfmlValue::strukt(env))
                }
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_stringbuilder(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" => {
            let init = args.first().map(|a| a.as_string()).unwrap_or_default();
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.lang.stringbuilder".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__buffer".to_string(), CfmlValue::string(init));
            Ok(CfmlValue::strukt(shim))
        }
        "append" => {
            if let CfmlValue::Struct(ref shim) = object {
                let cur = shim
                    .get("__buffer")
                    .map(|b| b.as_string())
                    .unwrap_or_default();
                let app = args.first().map(|a| a.as_string()).unwrap_or_default();
                let mut ns = shim.snapshot();
                ns.insert(
                    "__buffer".to_string(),
                    CfmlValue::string(format!("{}{}", cur, app)),
                );
                Ok(CfmlValue::strukt(ns))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "tostring" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(shim
                    .get("__buffer")
                    .unwrap_or(CfmlValue::string(String::new())))
            } else {
                Ok(CfmlValue::string(String::new()))
            }
        }
        "length" => {
            if let CfmlValue::Struct(ref shim) = object {
                let b = shim
                    .get("__buffer")
                    .map(|x| x.as_string())
                    .unwrap_or_default();
                Ok(CfmlValue::Int(b.len() as i64))
            } else {
                Ok(CfmlValue::Int(0))
            }
        }
        "clear" => {
            if let CfmlValue::Struct(ref shim) = object {
                let mut ns = shim.snapshot();
                ns.insert("__buffer".to_string(), CfmlValue::string(String::new()));
                Ok(CfmlValue::strukt(ns))
            } else {
                Ok(CfmlValue::Null)
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

// ---- TreeMap ----
pub fn handle_java_treemap(method: &str, args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    match method {
        "init" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.treemap".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            if let Some(CfmlValue::Struct(init)) = args.first() {
                for (k, v) in init.iter() {
                    shim.insert(k.clone(), v.clone());
                }
            }
            Ok(CfmlValue::strukt(shim))
        }
        "put" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some((k, v)) = args.get(0).zip(args.get(1)) {
                    let mut ns = shim.snapshot();
                    ns.insert(k.as_string(), v.clone());
                    Ok(CfmlValue::strukt(ns))
                } else {
                    Ok(object.clone())
                }
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "keyset" | "keys" => {
            if let CfmlValue::Struct(ref shim) = object {
                let mut ks: Vec<String> = shim
                    .iter()
                    .filter(|(k, _)| !k.starts_with("__"))
                    .map(|(k, _)| k.clone())
                    .collect();
                ks.sort(); // TreeMap = sorted key order
                Ok(CfmlValue::array(
                    ks.into_iter().map(CfmlValue::string).collect(),
                ))
            } else {
                Ok(CfmlValue::array(Vec::new()))
            }
        }
        "get" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(key) = args.first() {
                    let k = key.as_string();
                    return Ok(shim.get(&k).unwrap_or(CfmlValue::Null));
                }
            }
            Ok(CfmlValue::Null)
        }
        "size" | "len" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(CfmlValue::Int(
                    shim.iter().filter(|(k, _)| !k.starts_with("__")).count() as i64,
                ))
            } else {
                Ok(CfmlValue::Int(0))
            }
        }
        "containskey" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(key) = args.first() {
                    let k = key.as_string();
                    return Ok(CfmlValue::Bool(shim.contains_key(&k)));
                }
            }
            Ok(CfmlValue::Bool(false))
        }
        "isempty" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(CfmlValue::Bool(
                    shim.iter().all(|(k, _)| k.starts_with("__")),
                ))
            } else {
                Ok(CfmlValue::Bool(true))
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_linkedhashmap(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.linkedhashmap".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            Ok(CfmlValue::strukt(shim))
        }
        "keyset" | "keys" => {
            if let CfmlValue::Struct(ref shim) = object {
                let ks: Vec<CfmlValue> = shim
                    .iter()
                    .filter(|(k, _)| !k.starts_with("__"))
                    .map(|(k, _)| CfmlValue::string(k.clone()))
                    .collect();
                Ok(CfmlValue::array(ks))
            } else {
                Ok(CfmlValue::array(Vec::new()))
            }
        }
        "get" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(k)) = args.first() {
                    Ok(shim.get(k).unwrap_or(CfmlValue::Null))
                } else {
                    Ok(CfmlValue::Null)
                }
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "size" | "len" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(CfmlValue::Int(
                    shim.iter().filter(|(k, _)| !k.starts_with("__")).count() as i64,
                ))
            } else {
                Ok(CfmlValue::Int(0))
            }
        }
        "containskey" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(k)) = args.first() {
                    Ok(CfmlValue::Bool(shim.contains_key(k)))
                } else {
                    Ok(CfmlValue::Bool(false))
                }
            } else {
                Ok(CfmlValue::Bool(false))
            }
        }
        "put" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some((k, v)) = args.get(0).zip(args.get(1)) {
                    let mut ns = shim.snapshot();
                    ns.insert(k.as_string(), v.clone());
                    Ok(CfmlValue::strukt(ns))
                } else {
                    Ok(object.clone())
                }
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "isempty" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(CfmlValue::Bool(
                    shim.iter().all(|(k, _)| k.starts_with("__")),
                ))
            } else {
                Ok(CfmlValue::Bool(true))
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_concurrentlinkedqueue(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.concurrent.concurrentlinkedqueue".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__queue".to_string(), CfmlValue::array(Vec::new()));
            Ok(CfmlValue::strukt(shim))
        }
        "offer" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(item) = args.first() {
                    let mut ns = shim.snapshot();
                    if let Some(CfmlValue::Array(q)) = ns.get("__queue").cloned() {
                        let mut nq = q.snapshot();
                        nq.push(item.clone());
                        ns.insert("__queue".to_string(), CfmlValue::array(nq));
                    }
                    Ok(CfmlValue::strukt(ns))
                } else {
                    Ok(object.clone())
                }
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "poll" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::Array(q)) = shim.get("__queue") {
                    let qv = q.snapshot();
                    if !qv.is_empty() {
                        let mut ns = shim.snapshot();
                        let _itm = qv[0].clone();
                        let nq = qv[1..].to_vec();
                        ns.insert("__queue".to_string(), CfmlValue::array(nq));
                        return Ok(CfmlValue::strukt(ns));
                    }
                }
                Ok(CfmlValue::Null)
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "peek" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::Array(q)) = shim.get("__queue") {
                    if let Some(first) = q.first() {
                        return Ok(first);
                    }
                }
                Ok(CfmlValue::Null)
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "size" | "len" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::Array(q)) = shim.get("__queue") {
                    Ok(CfmlValue::Int(q.len() as i64))
                } else {
                    Ok(CfmlValue::Int(0))
                }
            } else {
                Ok(CfmlValue::Int(0))
            }
        }
        "isempty" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::Array(q)) = shim.get("__queue") {
                    return Ok(CfmlValue::Bool(q.is_empty()));
                }
                Ok(CfmlValue::Bool(true))
            } else {
                Ok(CfmlValue::Bool(true))
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

// ---- ConcurrentHashMap ----
// Preside/ColdBox Cachebox uses ConcurrentHashMap as a thread-safe cache
// pool: init, put, get, remove (returns old value), containsKey, size,
// keys() (fed into Collections.list), clear, isEmpty.
pub fn handle_java_concurrenthashmap(
    method: &str,
    args: Vec<CfmlValue>,
    object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.concurrent.concurrenthashmap".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            Ok(CfmlValue::strukt(shim))
        }
        "put" | "putifabsent" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some((k, v)) = args.get(0).zip(args.get(1)) {
                    let key = k.as_string();
                    // putIfAbsent is a no-op if key present
                    if method == "putifabsent" && shim.contains_key(&key) {
                        return Ok(object.clone());
                    }
                    let mut ns = shim.snapshot();
                    ns.insert(key, v.clone());
                    return Ok(CfmlValue::strukt(ns));
                }
                return Ok(object.clone());
            }
            Ok(CfmlValue::Null)
        }
        "get" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(k) = args.first() {
                    return Ok(shim.get(&k.as_string()).unwrap_or(CfmlValue::Null));
                }
            }
            Ok(CfmlValue::Null)
        }
        "containskey" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(k) = args.first() {
                    return Ok(CfmlValue::Bool(shim.contains_key(&k.as_string())));
                }
            }
            Ok(CfmlValue::Bool(false))
        }
        "size" | "len" => {
            if let CfmlValue::Struct(ref shim) = object {
                return Ok(CfmlValue::Int(
                    shim.iter().filter(|(k, _)| !k.starts_with("__")).count() as i64,
                ));
            }
            Ok(CfmlValue::Int(0))
        }
        "isempty" => {
            if let CfmlValue::Struct(ref shim) = object {
                return Ok(CfmlValue::Bool(
                    shim.iter().all(|(k, _)| k.starts_with("__")),
                ));
            }
            Ok(CfmlValue::Bool(true))
        }
        "keys" | "keyset" | "values" => {
            // keys() returns an Enumeration in real Java; keySet() returns a
            // Set. Callers typically either iterate or feed into
            // Collections.list(). Returning a CFML Array satisfies both —
            // arrayLen, indexing, and Collections.list() all work on it.
            if let CfmlValue::Struct(ref shim) = object {
                let values = method == "values";
                let items: Vec<CfmlValue> = shim
                    .iter()
                    .filter(|(k, _)| !k.starts_with("__"))
                    .map(|(k, v)| {
                        if values {
                            v.clone()
                        } else {
                            CfmlValue::string(k.clone())
                        }
                    })
                    .collect();
                return Ok(CfmlValue::array(items));
            }
            Ok(CfmlValue::array(Vec::new()))
        }
        "clear" => {
            if let CfmlValue::Struct(ref shim) = object {
                let mut ns = IndexMap::new();
                for (k, v) in shim.iter() {
                    if k.starts_with("__") {
                        ns.insert(k.clone(), v.clone());
                    }
                }
                return Ok(CfmlValue::strukt(ns));
            }
            Ok(CfmlValue::Null)
        }
        // remove is handled in the VM dispatch (needs return-and-mutate
        // semantics identical to Queue.poll); this arm is a no-op safety net.
        _ => Ok(CfmlValue::Null),
    }
}

// ---- Collections (static utility class) ----
// Preside/ColdBox use-case: Collections.list(map.keys()) converts a legacy
// Enumeration into an ArrayList. Since our ConcurrentHashMap.keys() already
// returns a CFML Array, Collections.list(array) is identity. We also handle
// a handful of other common static helpers so real code runs unchanged.
pub fn handle_java_collections(
    method: &str,
    args: Vec<CfmlValue>,
    _object: &CfmlValue,
) -> CfmlResult {
    match method {
        "init" => {
            // Collections is static-only; return a stub shim so static calls
            // dispatch through to this handler.
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.collections".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            Ok(CfmlValue::strukt(shim))
        }
        "list" => {
            // Collections.list(Enumeration) → ArrayList. Our callers hand in
            // a CFML Array already, so this is an identity operation.
            match args.into_iter().next() {
                Some(CfmlValue::Array(a)) => Ok(CfmlValue::Array(a)),
                Some(other) => Ok(other),
                None => Ok(CfmlValue::array(Vec::new())),
            }
        }
        "emptylist" | "emptyset" => Ok(CfmlValue::array(Vec::new())),
        "emptymap" => Ok(CfmlValue::strukt(IndexMap::new())),
        "unmodifiablelist" | "unmodifiableset" | "synchronizedlist" | "synchronizedset" => {
            // No true immutability in CFML; behave as identity like Lucee.
            match args.into_iter().next() {
                Some(v) => Ok(v),
                None => Ok(CfmlValue::array(Vec::new())),
            }
        }
        "unmodifiablemap" | "synchronizedmap" => match args.into_iter().next() {
            Some(v) => Ok(v),
            None => Ok(CfmlValue::strukt(IndexMap::new())),
        },
        "sort" => {
            if let Some(CfmlValue::Array(a)) = args.into_iter().next() {
                // Collections.sort mutates the list in place (reference semantics).
                a.with_write(|v| v.sort_by(|x, y| x.as_string().cmp(&y.as_string())));
                return Ok(CfmlValue::Array(a));
            }
            Ok(CfmlValue::Null)
        }
        "reverse" => {
            if let Some(CfmlValue::Array(a)) = args.into_iter().next() {
                a.with_write(|v| v.reverse());
                return Ok(CfmlValue::Array(a));
            }
            Ok(CfmlValue::Null)
        }
        _ => Ok(CfmlValue::Null),
    }
}

pub fn handle_java_paths(method: &str, args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    match method {
        "init" => {
            // Paths is a static-only class; return a stub shim so that
            // the subsequent .get(path) static call dispatches here.
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.nio.file.paths".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            Ok(CfmlValue::strukt(shim))
        }
        "get" => {
            let path = args.first().map(|a| a.as_string()).unwrap_or_default();
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.nio.file.paths".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__path".to_string(), CfmlValue::string(path));
            Ok(CfmlValue::strukt(shim))
        }
        "getparent" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    if let Some(p) = std::path::Path::new(path.as_str()).parent() {
                        let mut ps = IndexMap::new();
                        ps.insert(
                            "__java_class".to_string(),
                            CfmlValue::string("java.nio.file.paths".to_string()),
                        );
                        ps.insert("__java_shim".to_string(), CfmlValue::Bool(true));
                        ps.insert(
                            "__path".to_string(),
                            CfmlValue::string(p.to_string_lossy().to_string()),
                        );
                        return Ok(CfmlValue::strukt(ps));
                    }
                }
                Ok(CfmlValue::Null)
            } else {
                Ok(CfmlValue::Null)
            }
        }
        "isabsolute" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    return Ok(CfmlValue::Bool(std::path::Path::new(path.as_str()).is_absolute()));
                }
                Ok(CfmlValue::Bool(false))
            } else {
                Ok(CfmlValue::Bool(false))
            }
        }
        "tostring" => {
            if let CfmlValue::Struct(ref shim) = object {
                Ok(shim
                    .get("__path")
                    .unwrap_or(CfmlValue::string(String::new())))
            } else {
                Ok(CfmlValue::string(String::new()))
            }
        }
        "toabsolute" | "toabsolutepath" => {
            if let CfmlValue::Struct(ref shim) = object {
                if let Some(CfmlValue::String(path)) = shim.get("__path") {
                    let p = std::path::Path::new(path.as_str());
                    if p.is_absolute() {
                        return Ok(shim.get("__path").unwrap_or(CfmlValue::Null));
                    }
                    if let Ok(cwd) = std::env::current_dir() {
                        let full = cwd.join(path.as_str());
                        let mut ns = shim.snapshot();
                        ns.insert(
                            "__path".to_string(),
                            CfmlValue::string(full.to_string_lossy().to_string()),
                        );
                        return Ok(CfmlValue::strukt(ns));
                    }
                }
                Ok(CfmlValue::Null)
            } else {
                Ok(CfmlValue::Null)
            }
        }
        _ => Ok(CfmlValue::Null),
    }
}

fn rand_u128() -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    cfml_common::clock::now_unix_nanos().hash(&mut h);
    0x12345678u64.hash(&mut h);
    h.finish() as u128
}

/// Shim for `java.util.regex.Pattern` and the `Matcher` it produces — used by
/// Lucee apps for dynamic route matching. Backed by Rust's `regex` crate, whose
/// syntax is a close superset for the patterns these apps use.
///
/// Flow: `createObject("java","java.util.regex.Pattern")` → `init`; then
/// `.compile(regex)` → a compiled Pattern shim; `.matcher(input)` → a Matcher
/// shim. `find()`/`matches()`/`lookingAt()` advance the matcher's cursor and
/// stash the capture groups; they are handled inline in the VM (see
/// `java_matcher_step`) because they mutate matcher state that must be written
/// back to the variable. `group(n)`/`groupCount()` read that stashed state and
/// stay here (pure reads).
pub fn handle_java_pattern(method: &str, args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    use regex::Regex;

    let object_regex = || -> String {
        if let CfmlValue::Struct(s) = object {
            s.get("__regex").map(|v| v.as_string()).unwrap_or_default()
        } else {
            String::new()
        }
    };
    let compile = |pattern: &str| -> Result<Regex, CfmlError> {
        Regex::new(pattern).map_err(|e| {
            CfmlError::runtime(format!(
                "java.util.regex.Pattern: invalid pattern [{}]: {}",
                pattern, e
            ))
        })
    };

    match method {
        // createObject(...) with no pattern yet — an uncompiled Pattern handle.
        "init" => {
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.regex.pattern".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            Ok(CfmlValue::strukt(shim))
        }
        // Pattern.compile(regex[, flags]) — returns a compiled Pattern shim.
        "compile" => {
            let regex_str = args.first().map(|a| a.as_string()).unwrap_or_default();
            compile(&regex_str)?; // validate up front
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.regex.pattern".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__regex".to_string(), CfmlValue::string(regex_str));
            Ok(CfmlValue::strukt(shim))
        }
        "pattern" | "tostring" => Ok(CfmlValue::string(object_regex())),
        // Pattern.matcher(input) — create a Matcher positioned before the first
        // match. find()/matches()/lookingAt() (handled inline in the VM so they
        // can write the advanced state back) populate the capture groups.
        "matcher" => {
            let regex_str = object_regex();
            let input = args.first().map(|a| a.as_string()).unwrap_or_default();
            let re = compile(&regex_str)?;
            let group_count = re.captures_len() as i64 - 1;
            let mut shim = IndexMap::new();
            shim.insert(
                "__java_class".to_string(),
                CfmlValue::string("java.util.regex.matcher".to_string()),
            );
            shim.insert("__java_shim".to_string(), CfmlValue::Bool(true));
            shim.insert("__regex".to_string(), CfmlValue::string(regex_str));
            shim.insert("__input".to_string(), CfmlValue::string(input));
            shim.insert("__groupcount".to_string(), CfmlValue::Int(group_count));
            shim.insert("__matched".to_string(), CfmlValue::Bool(false));
            shim.insert("__findindex".to_string(), CfmlValue::Int(0));
            shim.insert("__groups".to_string(), CfmlValue::array(Vec::new()));
            Ok(CfmlValue::strukt(shim))
        }
        // Matcher.group([n]) — group 0 is the whole match.
        "group" => {
            if let CfmlValue::Struct(s) = object {
                let idx = args
                    .first()
                    .and_then(|a| a.as_string().trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if let Some(CfmlValue::Array(groups)) = s.get("__groups") {
                    return Ok(groups.snapshot().get(idx).cloned().unwrap_or(CfmlValue::Null));
                }
            }
            Ok(CfmlValue::Null)
        }
        "groupcount" => {
            if let CfmlValue::Struct(s) = object {
                return Ok(s.get("__groupcount").unwrap_or(CfmlValue::Int(0)));
            }
            Ok(CfmlValue::Int(0))
        }
        _ => Ok(CfmlValue::Null),
    }
}

/// Which matcher operation `java_matcher_step` performs.
pub enum MatchMode {
    /// `find()` — next non-overlapping match from the cursor; advances it.
    Find,
    /// `matches()` — the whole input must match; does not move the cursor.
    Matches,
    /// `lookingAt()` — match anchored at the start; does not move the cursor.
    LookingAt,
}

/// Advance a `java.util.regex.Matcher` shim one step. Returns
/// `(matched, updated_matcher)`: the updated struct carries the refreshed
/// `__groups`/`__matched` (and, for `Find`, the incremented `__findindex`) and
/// must be written back to the matcher variable so a subsequent `group(n)`
/// sees this step's captures. `find()` walks non-overlapping matches
/// left-to-right exactly like Java's `Matcher.find()`, so `while (m.find())`
/// terminates.
pub fn java_matcher_step(
    s: &cfml_common::dynamic::CfmlStruct,
    mode: MatchMode,
) -> Result<(bool, CfmlValue), CfmlError> {
    use regex::Regex;
    let regex_str = s.get("__regex").map(|v| v.as_string()).unwrap_or_default();
    let input = s.get("__input").map(|v| v.as_string()).unwrap_or_default();
    let re = Regex::new(&regex_str).map_err(|e| {
        CfmlError::runtime(format!(
            "java.util.regex.Matcher: invalid pattern [{}]: {}",
            regex_str, e
        ))
    })?;

    let find_index = s
        .get("__findindex")
        .and_then(|v| v.as_string().trim().parse::<usize>().ok())
        .unwrap_or(0);

    let caps = match mode {
        MatchMode::Find => re.captures_iter(&input).nth(find_index),
        MatchMode::Matches => re
            .captures(&input)
            .filter(|c| c.get(0).map(|m| m.start() == 0 && m.end() == input.len()).unwrap_or(false)),
        MatchMode::LookingAt => re
            .captures(&input)
            .filter(|c| c.get(0).map(|m| m.start() == 0).unwrap_or(false)),
    };

    let mut ns = s.snapshot();
    let matched = caps.is_some();
    let groups: Vec<CfmlValue> = match &caps {
        Some(caps) => (0..re.captures_len())
            .map(|i| {
                caps.get(i)
                    .map(|m| CfmlValue::string(m.as_str().to_string()))
                    .unwrap_or(CfmlValue::Null)
            })
            .collect(),
        None => Vec::new(),
    };
    ns.insert("__matched".to_string(), CfmlValue::Bool(matched));
    ns.insert("__groups".to_string(), CfmlValue::array(groups));
    if matches!(mode, MatchMode::Find) && matched {
        ns.insert("__findindex".to_string(), CfmlValue::Int((find_index + 1) as i64));
    }
    Ok((matched, CfmlValue::strukt(ns)))
}

// ===============================================================
// Servlet bridge: getPageContext().getRequest() / .getResponse()
// ===============================================================
//
// On Lucee and Adobe CF the page context exposes live servlet request/
// response objects in EVERY execution context — even CLI/task contexts,
// where Lucee synthesizes them (getRequestURL() returns
// "http://localhost/index.cfm" with no real HTTP request in sight).
// Wheels builds request URLs through this exact chain
// (`GetPageContext().getRequest().getRequestURL()`), so the bridge must be
// non-null and method-faithful in both serve and CLI mode.
//
// We model Lucee's behaviour (real servlet objects with the full
// HttpServletRequest/Response surface) rather than BoxLang's narrower
// FakePageContext (whose getRequest()/getResponse() return the page context
// itself). For broad compatibility the page-context shim also forwards the
// request-side accessors BoxLang exposes directly (getRequestURL et al.),
// making the surface a superset of both engines.
//
// Request values are synthesized from the request's CGI scope when present
// (serve mode); absent in bare CLI, we fall back to Lucee's task-context
// defaults (localhost:80, /index.cfm, GET, http). The response side is
// dispatched in `lib.rs` (it mutates `self.response_status` /
// `self.response_headers` so setStatus()/setHeader() are faithful in serve
// mode, not no-ops).

pub const SERVLET_PAGE_CONTEXT_CLASS: &str = "lucee.runtime.pagecontextimpl";
pub const SERVLET_REQUEST_CLASS: &str = "lucee.runtime.net.http.httpservletrequestwrap";
pub const SERVLET_RESPONSE_CLASS: &str = "lucee.runtime.net.http.httpservletresponsedummy";

/// Build the `HttpServletRequest` shim returned by getPageContext().getRequest().
/// `cgi` is the request's CGI scope (serve mode) or `None` in bare CLI.
pub fn build_servlet_request_shim(cgi: Option<&IndexMap<String, CfmlValue>>) -> CfmlValue {
    let nonempty = |k: &str| {
        cgi.and_then(|c| c.get(k))
            .map(|v| v.as_string())
            .filter(|s| !s.is_empty())
    };
    let server_name = nonempty("server_name").unwrap_or_else(|| "localhost".to_string());
    let port: i64 = nonempty("server_port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);
    let method = nonempty("request_method").unwrap_or_else(|| "GET".to_string());
    let script = nonempty("script_name").unwrap_or_else(|| "/index.cfm".to_string());
    let query = cgi
        .and_then(|c| c.get("query_string"))
        .map(|v| v.as_string())
        .unwrap_or_default();
    let remote = nonempty("remote_addr").unwrap_or_else(|| "127.0.0.1".to_string());
    let secure = cgi
        .and_then(|c| c.get("https"))
        .map(|v| v.as_string().eq_ignore_ascii_case("on"))
        .unwrap_or(false);
    let scheme = if secure { "https" } else { "http" };
    // Lucee omits the port from getRequestURL() when it is the scheme default.
    let port_part = if (!secure && port == 80) || (secure && port == 443) {
        String::new()
    } else {
        format!(":{}", port)
    };
    let request_url = format!("{}://{}{}{}", scheme, server_name, port_part, script);
    let content_type = nonempty("content_type");

    let mut s = IndexMap::new();
    s.insert("__java_shim".to_string(), CfmlValue::Bool(true));
    s.insert(
        "__java_class".to_string(),
        CfmlValue::string(SERVLET_REQUEST_CLASS.to_string()),
    );
    s.insert("__req_url".to_string(), CfmlValue::string(request_url));
    s.insert("__req_uri".to_string(), CfmlValue::string(script.clone()));
    s.insert("__req_query".to_string(), CfmlValue::string(query));
    s.insert("__req_method".to_string(), CfmlValue::string(method));
    s.insert("__req_scheme".to_string(), CfmlValue::string(scheme.to_string()));
    s.insert("__req_server_name".to_string(), CfmlValue::string(server_name));
    s.insert("__req_server_port".to_string(), CfmlValue::Int(port));
    s.insert("__req_servlet_path".to_string(), CfmlValue::string(script));
    s.insert("__req_remote_addr".to_string(), CfmlValue::string(remote));
    s.insert("__req_secure".to_string(), CfmlValue::Bool(secure));
    s.insert(
        "__req_content_type".to_string(),
        content_type.map(CfmlValue::string).unwrap_or(CfmlValue::Null),
    );
    // Retain the CGI snapshot so getHeader(name) can resolve http_* keys.
    if let Some(c) = cgi {
        s.insert("__req_cgi".to_string(), CfmlValue::strukt(c.clone()));
    }
    CfmlValue::strukt(s)
}

/// Build the `HttpServletResponse` shim. State lives on the VM
/// (`response_status`/`response_headers`); the shim itself is just a marker
/// dispatched in `lib.rs`.
pub fn build_servlet_response_shim() -> CfmlValue {
    let mut s = IndexMap::new();
    s.insert("__java_shim".to_string(), CfmlValue::Bool(true));
    s.insert(
        "__java_class".to_string(),
        CfmlValue::string(SERVLET_RESPONSE_CLASS.to_string()),
    );
    CfmlValue::strukt(s)
}

/// Build the page-context shim returned by getPageContext().
pub fn build_page_context_shim(cgi: Option<&IndexMap<String, CfmlValue>>) -> CfmlValue {
    let mut s = IndexMap::new();
    s.insert("__java_shim".to_string(), CfmlValue::Bool(true));
    s.insert(
        "__java_class".to_string(),
        CfmlValue::string(SERVLET_PAGE_CONTEXT_CLASS.to_string()),
    );
    s.insert("__pc_request".to_string(), build_servlet_request_shim(cgi));
    s.insert("__pc_response".to_string(), build_servlet_response_shim());
    CfmlValue::strukt(s)
}

/// Dispatch a method call on the `HttpServletRequest` shim. Read-only: every
/// value was synthesized at construction time, so this needs no VM access.
pub fn handle_servlet_request(method: &str, args: Vec<CfmlValue>, object: &CfmlValue) -> CfmlResult {
    let s = match object {
        CfmlValue::Struct(s) => s,
        _ => return Ok(CfmlValue::Null),
    };
    let get = |k: &str| s.get(k).unwrap_or(CfmlValue::Null);
    Ok(match method {
        "getrequesturl" => get("__req_url"),
        "getrequesturi" => get("__req_uri"),
        "getquerystring" => get("__req_query"),
        "getmethod" => get("__req_method"),
        "getscheme" => get("__req_scheme"),
        "getservername" => get("__req_server_name"),
        "getserverport" => get("__req_server_port"),
        "getservletpath" => get("__req_servlet_path"),
        "getremoteaddr" | "getremotehost" => get("__req_remote_addr"),
        "getcontenttype" => get("__req_content_type"),
        "issecure" => get("__req_secure"),
        "getprotocol" => CfmlValue::string("HTTP/1.1".to_string()),
        // Lucee serves apps at the context root, so contextPath is empty and
        // pathInfo is null for a plain script request.
        "getcontextpath" => CfmlValue::string(String::new()),
        "getpathinfo" => CfmlValue::Null,
        "getcharacterencoding" => CfmlValue::string("UTF-8".to_string()),
        "getlocaladdr" => get("__req_remote_addr"),
        "getlocalport" => get("__req_server_port"),
        "getheader" => {
            let name = args.first().map(|v| v.as_string()).unwrap_or_default();
            let key = format!("http_{}", name.to_lowercase().replace('-', "_"));
            match s.get("__req_cgi") {
                Some(CfmlValue::Struct(cgi)) => cgi.get(&key).unwrap_or(CfmlValue::Null),
                _ => CfmlValue::Null,
            }
        }
        // Unknown method: a non-null receiver is enough to keep call chains
        // alive; return null rather than throwing (matches a servlet getter
        // with no value).
        _ => CfmlValue::Null,
    })
}
