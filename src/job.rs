use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::config::DEFAULT_JOB_DELAY;
use crate::config::DEFAULT_JOB_PRIORITY;
use crate::error::BeanstalkcResult;
use crate::Beanstalkc;

/// `Job` is a simple abstraction about beanstalkd job.
#[derive(Debug)]
pub struct Job<'a> {
    conn: &'a mut Beanstalkc,
    id: u64,
    body: Vec<u8>,
    reserved: bool,
}

impl<'a> fmt::Display for Job<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Job(id: {}, reserved: {}, body: \"{:?}\")",
            self.id, self.reserved, self.body
        )
    }
}

impl<'a> Job<'a> {
    /// Initialize and return the `Job` object.
    pub fn new(conn: &'a mut Beanstalkc, job_id: u64, body: Vec<u8>, reserved: bool) -> Job {
        Job {
            conn,
            id: job_id,
            body,
            reserved,
        }
    }

    /// Return job id.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Return job body.
    pub fn body(&self) -> &[u8] {
        &self.body[..]
    }

    /// Return job reserving status.
    pub fn reserved(&self) -> bool {
        self.reserved
    }

    /// Delete this job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.delete().await.unwrap();
    /// }
    /// ```
    pub async fn delete(&mut self) -> BeanstalkcResult<()> {
        self.conn.delete(self.id).await?;
        self.reserved = false;
        Ok(())
    }

    /// Release this job back to the ready queue with default priority and delay.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.release_default().await.unwrap();
    /// }
    /// ```
    pub async fn release_default(&mut self) -> BeanstalkcResult<()> {
        let priority = self.priority().await;
        self.release(priority, DEFAULT_JOB_DELAY).await
    }

    /// Release this job back to the ready queue with custom priority and delay.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.release(0, Duration::from_secs(0)).await.unwrap();
    /// }
    /// ```
    pub async fn release(&mut self, priority: u32, delay: Duration) -> BeanstalkcResult<()> {
        if !self.reserved {
            return Ok(());
        }

        self.conn.release(self.id, priority, delay).await?;
        self.reserved = false;
        Ok(())
    }

    /// Bury this job with default priority.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.bury_default().await.unwrap();
    /// }
    /// ```
    pub async fn bury_default(&mut self) -> BeanstalkcResult<()> {
        let priority = self.priority().await;
        self.bury(priority).await
    }

    /// Bury this job with custom priority.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.bury(1024).await.unwrap();
    /// }
    /// ```
    pub async fn bury(&mut self, priority: u32) -> BeanstalkcResult<()> {
        if !self.reserved {
            return Ok(());
        }

        self.conn.bury(self.id, priority).await?;
        self.reserved = false;
        Ok(())
    }

    /// Kick this job to ready queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek_buried().await.unwrap();
    /// job.kick().await.unwrap();
    /// }
    /// ```
    pub async fn kick(&mut self) -> BeanstalkcResult<()> {
        self.conn.kick_job(self.id).await
    }

    /// Touch this reserved job, requesting more time to work on it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.reserve().await.unwrap();
    /// job.touch().await.unwrap();
    /// }
    /// ```
    pub async fn touch(&mut self) -> BeanstalkcResult<()> {
        if !self.reserved {
            return Ok(());
        }

        self.conn.touch(self.id).await
    }

    /// Return a dict of statistical information about this job.
    ///
    /// # Example
    ///
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() {
    /// use std::time::Duration;
    /// use beanstalkc::Beanstalkc;
    ///
    /// let mut conn = Beanstalkc::new().connect().await.unwrap();
    ///
    /// let mut job = conn.peek_ready().await.unwrap();
    /// let job_stats = job.stats().await.unwrap();
    /// dbg!(job_stats);
    /// }
    /// ```
    pub async fn stats(&mut self) -> BeanstalkcResult<HashMap<String, String>> {
        self.conn.stats_job(self.id).await
    }

    /// Return the job priority from this job stats. If not found, return the `DEFAULT_JOB_PRIORITY`.
    async fn priority(&mut self) -> u32 {
        let stats = self.stats().await.unwrap_or_default();
        stats
            .get("pri")
            .map(|x| x.parse().unwrap_or(DEFAULT_JOB_PRIORITY))
            .unwrap_or(DEFAULT_JOB_PRIORITY)
    }
}
