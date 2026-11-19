pub(super) fn defaults(name: &str) -> (u32, &'static str) {
    let name = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| name.contains(word));
    if has(&[
        "red bull", "redbull", "sting", "predator", "armour", "monster", "energy", "non stop",
        "nonstop", "dragon", "hell",
    ]) {
        (4000, "22029090")
    } else if has(&[
        "smoodh",
        "cocklate",
        "chocolate",
        "cocoa",
        "badam",
        "lassi",
        "chaach",
        "chach",
        "dahi",
        "curd",
        "butter",
        "nescafe",
        "coffee",
        "kesar",
        "milk",
    ]) {
        (500, "22029930")
    } else if has(&["appy fizz", "lemonata", "limonata"])
        || (name.contains("lahori") && !name.contains("aam"))
    {
        (4000, "22029090")
    } else if name.contains("soda") {
        (1800, "22011010")
    } else if has(&[
        "aquafina",
        "bisleri",
        "kinley",
        "bailley",
        "sure",
        "indus",
        "ind-",
        "coco water",
        "water",
    ]) {
        (500, "22011010")
    } else if has(&[
        "maaza",
        "slice",
        "tropicana",
        "frooti",
        "real",
        "pulpy",
        "minute",
        "nimbooz",
        "nimboo",
        "aam ras",
        "aamras",
        "mango",
        "sun crush",
        "raskik",
        "fresca",
        "fc ",
        "guava",
        "litchi",
        "catch juice",
        "juice",
        "appy",
    ]) {
        (500, "22029920")
    } else if has(&["kurkure", "lays", "chips", "uncle", "namkeen"]) {
        (500, "21069099")
    } else {
        (4000, "22029090")
    }
}
