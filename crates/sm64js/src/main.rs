use rustrict::{add_word, Type};

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    unsafe {
        add_word("crap", Type::SAFE);
        add_word("damn", Type::SAFE);
        add_word("dic", Type::SAFE);
        add_word("hell", Type::SAFE);
    }
    sm64js::main()?.await
}
