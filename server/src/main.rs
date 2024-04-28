use std::net::SocketAddr;
use std::ptr::addr_of;

use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use tokio::net::TcpListener;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use lol_html::{element, HtmlRewriter, Settings};
use mini_moka::sync::Cache;

use std::thread;

fn value(n: usize) -> String {
    format!("value {}", n)
}

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            // Ignore this line for now.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => {
            let mut output = vec![];

            let mut rewriter = HtmlRewriter::new(
                Settings {
                    element_content_handlers: vec![
                        // Rewrite insecure hyperlinks
                        element!("a[href]", |el| {
                            let href = el.get_attribute("href").unwrap().replace("http:", "https:");

                            el.set_attribute("href", &href).unwrap();

                            Ok(())
                        }),
                    ],
                    ..Settings::default()
                },
                |c: &[u8]| output.extend_from_slice(c),
            );

            rewriter.write(b"<div><a href=").unwrap();
            rewriter.write(b"http://example.com>").unwrap();
            rewriter.write(b"</a></div>").unwrap();
            rewriter.end().unwrap();
            // let when = Instant::now() + Duration::from_millis(1000);
            // let future = Delay { when };
            // let out = future.await;
            Ok(Response::new(Body::from(
            "Try POSTing data to /echo such as: `curl localhost:8080/echo -XPOST -d 'hello world'`".to_owned() + String::from_utf8(output).unwrap().as_str(),
        )))
        }

        // Simply echo the body back to the client.
        (&Method::POST, "/echo") => Ok(Response::new(req.into_body())),

        (&Method::POST, "/echo/reversed") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await?;

            let reversed_body = whole_body.iter().rev().cloned().collect::<Vec<u8>>();
            Ok(Response::new(Body::from(reversed_body)))
        }

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let cache = Cache::new(10_000);
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let my_cache = cache.clone();
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            my_cache.insert("test", "test");
            if let Err(err) = Http::new().serve_connection(stream, service_fn(echo)).await {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
