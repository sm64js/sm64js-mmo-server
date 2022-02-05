use rustrict::{add_word, Type};

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    unsafe {
        add_word("butt", Type::SAFE);
        add_word("crap", Type::SAFE);
        add_word("damn", Type::SAFE);
        add_word("dic", Type::SAFE); // DiC entertainment
        add_word("frick", Type::SAFE);
        add_word("hell", Type::SAFE);
        add_word("naked", Type::SAFE);
        add_word("gay", Type::SAFE); // lets hope these aren't misused...
        add_word("homo", Type::SAFE);
    }
    sm64js::main()?.await
}
