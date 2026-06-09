use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::web::Data;

//WHY a custom alphabet rspamd? WHY?!?
static BASE64: data_encoding::Encoding = data_encoding_macro::new_encoding!{
    symbols: "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz+/",
    padding: '=',
};

#[actix_web::routes]
#[get("/selector_map")]
#[head("/selector_map")]
pub async fn selector_map(db: Data<sqlx::postgres::PgPool>) -> HttpResponse<String> {
    let res = sqlx::query!(r#"
SELECT public.flattened_domains.name as "name!", public.dkim.selector from dkim
INNER JOIN public.flattened_domains ON public.dkim.domain_id = public.flattened_domains.id
WHERE public.dkim.active"#)
        .fetch_all(db.get_ref())
        .await;

    let res = match res {
        Err(err) => {
            log::error!("Error fetching DKIM: {err}");
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}"));
        },
        Ok(v) => v,
    };

    let res = String::from_iter(res.into_iter().map(|v|format!("{} {}\n", v.name, v.selector)));

    HttpResponse::with_body(StatusCode::OK, res)
}

#[actix_web::routes]
#[get("/signing_table")]
#[head("/signing_table")]
pub async fn signing_table(db: Data<sqlx::postgres::PgPool>) -> HttpResponse<String> {
    let res = sqlx::query!(r#"
SELECT public.flattened_domains.name as "name!", public.dkim.domain_id, public.dkim.selector from dkim
INNER JOIN public.flattened_domains ON public.dkim.domain_id = public.flattened_domains.id
WHERE public.dkim.active"#)
        .fetch_all(db.get_ref())
        .await;

    let res = match res {
        Err(err) => {
            log::error!("Error fetching DKIM: {err}");
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}"));
        },
        Ok(v) => v,
    };

    let res = String::from_iter(res.into_iter().map(|v|format!("*@{domain} {domain}-{selector}\n", domain=v.name, selector=v.selector)));

    HttpResponse::with_body(StatusCode::OK, res)
}

#[actix_web::routes]
#[get("/key_table")]
#[head("/key_table")]
pub async fn key_table(db: Data<sqlx::postgres::PgPool>) -> HttpResponse<String> {
    let res = sqlx::query!(r#"
SELECT public.flattened_domains.name as "name!", public.dkim.domain_id, public.dkim.selector, public.dkim.private_key from dkim
INNER JOIN public.flattened_domains ON public.dkim.domain_id = public.flattened_domains.id
WHERE public.dkim.active"#)
        .fetch_all(db.get_ref())
        .await;

    let res = match res {
        Err(err) => {
            log::error!("Error fetching DKIM: {err}");
            return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}"));
        },
        Ok(v) => v,
    };

    let res = String::from_iter(res.into_iter().map(|v|format!("{domain}-{selector} {domain}:{selector}:{}\n", BASE64.encode(v.private_key.as_slice()), domain=v.name, selector=v.selector)));

    HttpResponse::with_body(StatusCode::OK, res)
}