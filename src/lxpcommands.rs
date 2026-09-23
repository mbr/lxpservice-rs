use std::{fs, io::Write, path::PathBuf, sync::mpsc::channel};

use futures::{stream, StreamExt};
use log::{debug, error, info, trace};
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};

use crate::{lxpapi, lxpconfig, lxptypes};

#[derive(Debug, Clone)]
pub struct LxpCommands {
    config: lxpconfig::LxpConfig,
    api_ref: Option<lxpapi::LxpApi>,
}

impl LxpCommands {
    pub fn new(config_dir: &PathBuf) -> LxpCommands {
        let config = lxpconfig::LxpConfig::new(config_dir);
        LxpCommands {
            config,
            api_ref: None,
        }
    }

    fn api(&mut self) -> lxpapi::LxpApi {
        match &self.api_ref {
            Some(_api) => (),
            None => {
                // Get profile and instanciate api
                let profile = self.config.get_active_profile().unwrap();
                self.api_ref = Some(lxpapi::LxpApi::new(
                    &profile.user_name,
                    &profile.api_key,
                    &profile.url,
                ))
            }
        };
        self.api_ref.clone().unwrap()
    }

    pub fn profile_new(&mut self, profile_name: &str, user_name: &str, url: &str, api_key: &str) {
        info!(
            "New profile {}, user '{}', url '{}' and <api_key>",
            profile_name, user_name, url
        );
        info!("Active profile is set to '{}'", profile_name);
        let profile = lxpconfig::Profile {
            user_name: user_name.into(),
            url: url.into(),
            api_key: api_key.into(),
        };
        self.config.new_profile(profile_name, profile);
    }

    pub fn profile_delete(&mut self, profile_name: &str) {
        self.config.delete_profile(profile_name);
    }

    pub fn profile_delete_all(&mut self) {
        self.config.delete_all_profiles();
    }

    pub fn profile_switch(&mut self, profile_name: &str) {
        self.config.switch_profile(profile_name);
    }

    pub fn profile_show(&mut self) {
        self.config.show_profiles();
    }

    fn _invoice_write_pdf_file(&self, r: lxptypes::Response, pdf_file: Vec<u8>) {
        let profile_name = self.config.get_active_profile_name().unwrap();
        match &r.invoice {
            Some(invoice) => {
                let file_name: String =
                    format!("{}_{}-invoice.pdf", invoice.invoicedate, &profile_name);
                info!("Writing file '{}'", file_name);
                let mut buffer = fs::File::create(file_name).expect("Could not create PDF file");
                buffer
                    .write_all(&pdf_file)
                    .expect("Could not write PDF file");
            }
            None => error!("<No data>"),
        }
    }

    pub async fn invoice_list(&mut self) {
        match self.api().list_invoices().await {
            Ok(r) => match &r.invoices {
                Some(invoices) => {
                    info!("\n{:<10} {:>6} {:>8}", "Date", "Id", "Cost");
                    for (_key, invoice) in invoices {
                        let cost = invoice.sum.parse::<f64>().unwrap()
                            + invoice.vat.parse::<f64>().unwrap();
                        info!(
                            "{:<10} {:>6} {:>6.2} €",
                            &invoice.invoicedate, &invoice.iid, &cost,
                        )
                    }
                }
                None => info!("<No data>"),
            },
            Err(e) => error!("Error when getting invoice list {}", e),
        }
    }

    pub async fn invoice_get_last(&mut self) {
        match self.api().get_last_invoice().await {
            Ok(r) => self._invoice_write_pdf_file(r.0, r.1),
            Err(e) => error!("Error when getting invoice {}", e),
        }
    }

    pub async fn invoice_get_by_id(&mut self, id: &str) {
        match id.parse::<i32>() {
            Ok(id) => {
                debug!("Storing invoice, ID: {}", id);
                match self.api().get_invoice(id).await {
                    Ok(r) => self._invoice_write_pdf_file(r.0, r.1),
                    Err(e) => error!("Error when getting invoice {}", e),
                }
            }
            Err(e) => {
                error!("Invoice id must be Integer: Error Message '{}'", e);
            }
        }
    }

    fn _job_show_list(&self, r: lxptypes::Response) {
        match &r.jobs {
            Some(jobs) => {
                let mut sum_cost: f64 = 0.0;
                info!(
                    "\n{:<10} {:>8} {:>3} {:>3} {:>3} {:>3} {:>4} {:<35}",
                    "Date", "Id", "Pgs", "Col", "Dpx", "Shp", "Cost", "Filename"
                );
                for (_key, job) in jobs {
                    let cost =
                        job.cost.parse::<f64>().unwrap() + job.cost_vat.parse::<f64>().unwrap();
                    sum_cost += cost;
                    info!(
                        "{:<10} {:>8} {:>3} {:>3} {:>3} {:>3} {:>4.2} {:<35}",
                        &job.date[..10],
                        &job.jid,
                        &job.pages,
                        &job.color,
                        &job.mode[..3],
                        &job.shipping[..3],
                        &cost,
                        &job.address
                    )
                }
                info!("The sum of the costs is {:.2} €", sum_cost)
            }
            None => info!("<No data>"),
        }
    }

    async fn _job_show_lists(&mut self) -> Result<(), lxpapi::LxpApiError> {
        let r = self.api().get_blance().await?;
        info!("Credit balance {} €", r.balance.unwrap().value);

        debug!("Check the status of the placed print jobs");
        let r = self.api().get_jobs_queue(7).await?;
        info!("\nThese letters will be sent soon:");
        self._job_show_list(r);

        let r = self.api().get_jobs_hold().await?;
        info!("\nThese letters are in the queue (credit exhausted):");
        self._job_show_list(r);

        let r = self.api().get_jobs_sent(7).await?;
        info!("\nThese letters are sent in the last 7 days:");
        self._job_show_list(r);
        Ok(())
    }

    pub async fn job_overview(&mut self) {
        info!(
            "Active profile '{}'",
            match self.config.get_active_profile_name() {
                Some(user) => user,
                None => String::from("<No active profile>"),
            }
        );

        match self._job_show_lists().await {
            Ok(()) => (),
            Err(e) => error!("Error in rest service {}", e),
        }
    }

    async fn _job_delete_by_id(&mut self, id: i32, file_name: &str) {
        match self.api().delete_job(id).await {
            Ok(r) => match r.status {
                200 => info!("  Job id {} {} deleted", id, file_name),
                404 => error!("Job Id {} not found", id),
                _ => error!("Don't know what to do with status {}", r.status),
            },
            Err(e) => error!("Error in server connection {}", e),
        }
    }

    async fn _jobs_delete_list(&mut self, r: lxptypes::Response) -> i32 {
        let mut jobs_deleted: i32 = 0;
        match &r.jobs {
            Some(jobs) => {
                for (_key, job) in jobs {
                    let id = job
                        .jid
                        .parse::<i32>()
                        .expect("Job id must be integer, error in JSON string");
                    self._job_delete_by_id(id, &job.address).await;
                    jobs_deleted += 1;
                }
            }
            None => (),
        }
        jobs_deleted
    }

    pub async fn job_delete_all(&mut self) {
        let mut jobs_deleted: i32 = match self.api().get_jobs_queue(7).await {
            Ok(r) => self._jobs_delete_list(r).await,
            Err(e) => {
                error!("{}", e);
                0
            }
        };

        jobs_deleted += match self.api().get_jobs_hold().await {
            Ok(r) => self._jobs_delete_list(r).await,
            Err(e) => {
                error!("{}", e);
                0
            }
        };
        info!("{} job(s) deleted", jobs_deleted)
    }

    pub async fn job_delete_by_id(&mut self, id_arg: &str) {
        let id = match id_arg.parse::<i32>() {
            Ok(id) => {
                debug!("Deleting a single print job on server, ID: {}", id);
                id
            }
            Err(e) => {
                error!("Deleting id must be Integer: Error Message '{}'", e);
                0
            }
        };
        self._job_delete_by_id(id, "").await;
    }

    pub async fn job_set_file_or_dir(
        &mut self,
        file_or_dir_name: &str,
        color: lxptypes::ColorPrint,
        mode: lxptypes::Mode,
        ship: lxptypes::Ship,
    ) {
        match std::fs::metadata(file_or_dir_name) {
            Ok(md) => {
                if md.is_file() {
                    match self
                        .api()
                        .set_job(&file_or_dir_name, &color, &mode, &ship)
                        .await
                    {
                        Ok(_r) => info!("  Job {} sent", &file_or_dir_name),
                        Err(_) => (), // Error message was already issued by set_job()
                    }
                };
                if md.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(file_or_dir_name) {
                        let api = &self.api();
                        let puts = stream::iter(entries.into_iter().map(|entry| {
                            async move {
                                if let Ok(entry) = entry {
                                    let path = entry.path();
                                    if path.is_file() {
                                        let p = path.to_str().unwrap();
                                        match api.set_job(&p, &color, &mode, &ship).await {
                                            Ok(_r) => info!("  Job {} sent", &p),
                                            Err(_) => (), // Error message was already issued by set_job()
                                        }
                                    }
                                }
                            }
                        }))
                        .buffer_unordered(5)
                        .collect::<Vec<()>>(); // up to 5 concurrent async requests
                        puts.await;
                    }
                }
            }
            Err(e) => error!("Opening send file: {}", e),
        };
    }
    pub async fn watch_dir(
        &mut self,
        dir_name: &PathBuf,
        color: lxptypes::ColorPrint,
        mode: lxptypes::Mode,
        ship: lxptypes::Ship,
    ) {
        debug!("Watch directory '{:#?}' for new PDF files", &dir_name);
        let watch_dir = std::path::Path::new(&dir_name);
        match fs::create_dir_all(&watch_dir) {
            Ok(_) => (),
            Err(e) => error!("Could not create watch_dir {:#?}, error {}", &watch_dir, e),
        }

        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        let mut watcher = recommended_watcher(tx).expect("filesystem watcher is available");

        // Add a path to be watched and monitored for changes.
        match watcher.watch(&dir_name, RecursiveMode::NonRecursive) {
            Ok(_) => (),
            Err(e) => error!("Couldn't watch '{:#?}', error {}", &dir_name, e),
        };

        loop {
            let pdf_path = match rx.recv() {
                Ok(Ok(event)) if matches!(event.kind, EventKind::Create(_)) => {
                    event.paths.into_iter().find(|path| {
                        path.extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
                    })
                }
                Ok(_) => None,
                Err(e) => {
                    trace!("watch error: {:?}", e);
                    None
                }
            };

            match pdf_path {
                Some(from_path) => {
                    // push pdf file to print service
                    match self
                        .api()
                        .set_job(from_path.to_str().unwrap(), &color, &mode, &ship)
                        .await
                    {
                        Ok(_r) => info!("File {:#?} sent", &from_path),
                        Err(_) => (), // Error message was already issued by set_job()
                    }

                    // move pdf filt to sent directory
                    let file_name = from_path.file_name().unwrap();
                    let to_path = from_path.parent().unwrap().join("sent").join(&file_name);
                    match fs::rename(&from_path, &to_path) {
                        Ok(_) => trace!("Move {:#?} to directory sent", &from_path),
                        Err(e) => error!("Could not move PDF file {}", e),
                    };
                }
                None => (),
            }
        }
    }
}
