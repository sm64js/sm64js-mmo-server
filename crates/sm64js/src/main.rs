use rustrict::{add_word, Type};

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    unsafe {
        add_word("can't it", Type::SAFE);
        add_word("cant it", Type::SAFE);
        add_word("butt", Type::SAFE);
        add_word("crap", Type::SAFE);
        add_word("damn", Type::SAFE);
        add_word("dic", Type::SAFE); // DiC entertainment
        add_word("fat", Type::SAFE);
        add_word("frick", Type::SAFE);
        add_word("isgay", Type::SAFE);
        add_word("gay", Type::SAFE);
        add_word("hell", Type::SAFE);
        add_word("to hell", Type::SAFE);
        add_word("h i liter", Type::SAFE);
        add_word("hoe", Type::SAFE);
        add_word("homo", Type::SAFE);
        add_word("naked", Type::SAFE);
        add_word("naughty", Type::SAFE);
        add_word("poggers", Type::SAFE);
        add_word("splix", Type::SAFE);
        add_word("suck", Type::SAFE);
        add_word("sucks", Type::SAFE);
        add_word("stfu", Type::SAFE);
        add_word("to hell", Type::SAFE);
        add_word("ur mom", Type::SAFE);

        add_word("mierda", Type::PROFANE);
    }
    sm64js::main()?.await
}
