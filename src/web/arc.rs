use actix_web::body::MessageBody;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::web::Data;

#[actix_web::get("/signing_table")]
pub async fn signing_table(db: Data<sqlx::postgres::PgPool>) -> HttpResponse {
    let res = sqlx::query!(r#"
SELECT public.flattened_domains.name as "name!", public.dkim.domain_id, public.dkim.selector from dkim
INNER JOIN public.flattened_domains ON public.dkim.domain_id = public.flattened_domains.id
WHERE public.dkim.active"#)
        .fetch_all(db.get_ref())
        .await;

    let res = match res {
        Err(err) => {
            log::error!("Error fetching DKIM: {err}");
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}").boxed());
        },
        Ok(v) => v,
    };

    let res = String::from_iter(res.into_iter().map(|v|format!("*@{} {}-{}\n", v.name, v.domain_id, v.selector)));

    HttpResponse::with_body(StatusCode::OK, res.boxed())
}
#[actix_web::get("/key_table")]
pub async fn key_table(db: Data<sqlx::postgres::PgPool>) -> HttpResponse {
    let res = sqlx::query!(r#"
SELECT public.flattened_domains.name as "name!", public.dkim.domain_id, public.dkim.selector, public.dkim.private_key from dkim
INNER JOIN public.flattened_domains ON public.dkim.domain_id = public.flattened_domains.id
WHERE public.dkim.active"#)
        .fetch_all(db.get_ref())
        .await;

    let res = match res {
        Err(err) => {
            log::error!("Error fetching DKIM: {err}");
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}").boxed());
        },
        Ok(v) => v,
    };

    let res = String::from_iter(res.into_iter().map(|v|format!("{}-{} {} {} {}\n",v.domain_id, v.selector, v.name, v.selector, data_encoding::BASE64.encode(v.private_key.as_slice()))));

    HttpResponse::with_body(StatusCode::OK, res.boxed())
}