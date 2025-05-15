use once_cell::sync::Lazy;
use reqwest::Client;
use std::env;
use testcontainers::{
    ContainerAsync, GenericImage,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

/// Test environment setup
/// This struct is used to manage the test environment for the tests.
/// It contains the container, client, and URL for the test server.
/// The container is started with the specified image and tag,
/// and the client is used to send requests to the server.
/// The URL is constructed based on the exposed port and listen string.
/// The container is automatically stopped and removed when it goes out of scope.
/// We intentionally ignore the dead code warning here in regard to container.
#[allow(dead_code)]
pub(crate) struct TestEnv {
    container: testcontainers::core::error::Result<ContainerAsync<GenericImage>>,
    pub(crate) client: Client,
    pub(crate) url: String,
}

impl TestEnv {
    /// Creates a new test environment.
    /// TODO: investgate "associated function in this implementation warning. Allowing for now"
    ///
    #[allow(dead_code)]
    pub(crate) async fn new(path: &str) -> Self {
        let (container, client, url) = setup_test_environment(
            IMAGE.as_str(),
            TAG.as_str(),
            EXPOSED_PORT.parse::<u16>().unwrap(),
            &LISTEN_STRING,
            path,
        )
        .await;
        Self {
            container,
            client,
            url,
        }
    }
}


impl Drop for TestEnv {
    fn drop(&mut self) {
        // Custom cleanup logic can be added here if needed
        // For now, we just print a message indicating the test environment is being cleaned up
        println!("Cleaning up test environment...");
    }
}

pub async fn setup_test_environment(
    image: &str,
    tag: &str,
    exposed_port: u16,
    listen_str: &str,
    test_uri: &str,
) -> (
    testcontainers::core::error::Result<ContainerAsync<GenericImage>>,
    Client,
    String,
) {
    let container = GenericImage::new(image, tag)
        .with_exposed_port(exposed_port.tcp())
        .with_wait_for(WaitFor::message_on_stdout(listen_str))
        .start()
        .await;

    let client = Client::new();
    let host_port = container
        .as_ref()
        .unwrap()
        .get_host_port_ipv4(exposed_port)
        .await
        .unwrap();
    let url = format!("http://localhost:{}{}", host_port, test_uri);

    (container, client, url)
}

pub static IMAGE: Lazy<String> =
    Lazy::new(|| env::var("TEST_IMAGE").unwrap_or_else(|_| "microscaler/sesame-prism".to_string()));
pub static TAG: Lazy<String> = Lazy::new(|| env::var("TEST_TAG").unwrap_or_else(|_| "latest".to_string()));
pub static EXPOSED_PORT: Lazy<String> = Lazy::new(|| {
    env::var("TEST_PORT")
        .unwrap_or_else(|_| "4010".to_string())
        .parse()
        .expect("Invalid port number")
});

pub static LISTEN_STRING: Lazy<String> = Lazy::new(|| {
    if EXPOSED_PORT.as_str() == "4010" {
        "Prism is listening on".to_string()
    } else {
        "Server started successfully".to_string()
    }
});
