use crate::benchmark::BenchmarkCommands;
use crate::database::DatabaseInterface;
use crate::error::VerifierResult;
use crate::request::{get_response_body, get_response_headers, ContentType};
use crate::test_type::query::Query;
use crate::test_type::Executor;
use crate::verification::Messages;
use std::cmp::min;

pub struct CachedQuery {
    pub concurrency_levels: Vec<u32>,
    pub database_verifier: Box<dyn DatabaseInterface>,
}
impl Query for CachedQuery {}
impl Executor for CachedQuery {
    fn wait_for_database_to_be_available(&self) {
        self.database_verifier.wait_for_database_to_be_available();
    }

    fn retrieve_benchmark_commands(&self, url: &str) -> VerifierResult<BenchmarkCommands> {
        let primer_command = self.get_wrk_command(url, 5, 8);
        let warmup_command =
            self.get_wrk_command(url, 15, *self.concurrency_levels.iter().max().unwrap());
        let mut benchmark_commands = Vec::default();
        for concurrency in &self.concurrency_levels {
            benchmark_commands.push(self.get_wrk_command(url, 15, *concurrency));
        }

        Ok(BenchmarkCommands {
            primer_command,
            warmup_command,
            benchmark_commands,
        })
    }

    fn verify(&self, url: &str) -> VerifierResult<Messages> {
        let mut messages = Messages::new(url);

        // Initialization for query counting
        let repetitions = 2;
        let concurrency = *self.concurrency_levels.iter().max().unwrap();
        let expected_queries = 20 * repetitions * concurrency;
        let _expected_rows = expected_queries;

        // Because this test type is going to make a LOT of requests with a reasonably long timeout,
        // we use `get_response_headers` as a sentinel. If a `CurlError` is thrown, then we do not
        // perform any of the follow-up requests to conserve time.
        if let Ok(response_headers) = get_response_headers(&url, &mut messages) {
            messages.headers(&response_headers);
            self.verify_headers(&response_headers, &url, ContentType::Json, &mut messages);

            let test_cases = ["2", "0", "foo", "501", ""];
            let min = 1;
            let max = 500;

            for test_case in test_cases.iter() {
                let expected_length = self.translate_query_count(*test_case, min, max);
                let url = format!("{}{}", url, test_case);

                if let Some(response_body) = get_response_body(&url, &mut messages) {
                    messages.body(&response_body);
                    self.verify_with_length(&response_body, expected_length, &mut messages);
                }
            }
        }

        Ok(messages)
    }
}
impl CachedQuery {
    fn get_wrk_command(&self, url: &str, duration: u32, concurrency: u32) -> Vec<String> {
        vec![
            "wrk",
            "-H",
            "Host: bw-server",
            "-H",
            "Accept: application/json,text/html;q=0.9,application/xhtml+xml;q=0.9,application/xml;q=0.8,*/*;q=0.7",
            "-H",
            "Connection: keep-alive",
            "--latency",
            "-d",
            &format!("{}", duration),
            "-c",
            &format!("{}", concurrency),
            "--timeout",
            "8",
            "-t",
            &format!("{}", min(concurrency, num_cpus::get() as u32)),
            url,
        ].iter().map(|item| item.to_string()).collect()
    }
}
