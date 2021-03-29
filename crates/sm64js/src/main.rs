#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    sm64js::main()?.await
}
