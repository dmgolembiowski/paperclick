use fantoccini::{Client, ClientBuilder, Locator};
use shutdown_handler::{ShutdownHandler, SignalOrComplete};
use std::process;
use std::sync::Arc;
use std::task::Poll;
use std::{thread, time};
use tokio::runtime::Runtime;
use tokio::task;
use tokio::time::sleep;

fn main() {
    let join_handle = std::thread::spawn(|| {
        process::Command::new("geckodriver")
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .spawn()
            .expect("`geckodriver` not found on PATH");
    });

    let runtime = Runtime::new().unwrap();
    let shutdown = std::sync::Arc::new(ShutdownHandler::new());

    runtime
        .block_on(async move {
            let sd = Arc::clone(&shutdown);
            let result = fantoccini_main(sd).await;

            task::spawn_blocking(move || {
                process::Command::new("pkill")
                    .arg("geckodriver")
                    .spawn()
                    .expect("Could not kill geckodriver");
                join_handle.join();
            });

            result
        })
        .unwrap();
}

async fn fantoccini_main(
    shutdown: std::sync::Arc<ShutdownHandler>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to webdriver instance that is listening on port 4444
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?;

    // Go to the Rust website.
    client
        .goto("https://www.decisionproblem.com/paperclips/index2.html")
        .await?;

    // Click the "Get Started" button.
    let button = client
        .wait()
        .for_element(Locator::Css("#btnMakePaperclip"))
        .await?;

    let it = std::pin::pin!(async move {
        loop {
            button.click().await;
        }
    });
    match shutdown.wait_for_signal_or_future(it).await {
        SignalOrComplete::Completed(_) => {}
        SignalOrComplete::ShutdownSignal(it) => {
            it.await;
        }
    }

    client.close().await?;

    Ok(())
}
