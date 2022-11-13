use clap::{Parser};
use std::{thread::{self},time};

#[derive(Parser, Debug)]
#[clap(author="Kato", version, about="Download image tiles from a webserver. Replaces {x}, {y} in url with tile-number, {z} with zoom-level, and {bounds} with a bounding-box. Reprojection is not supported")]
struct Args {
	#[clap(short='u',long)]
	/// Example: http://maps/{Z}/{X}/{Y}.png or https://map?bbox={bounds}
	url: String,

	#[clap(default_value=".",short='o',long)]
	/// Tiles saved here in directories Z/X/Y
	output_dir: String,

	#[clap(default_value="0",short='s',long)]
	/// Start zoom-level. Inclusive
	start_zoom: u32,

	#[clap(short='e',long)]
	/// End zoom-level. Inclusive
	end_zoom: u32,

	#[clap(default_value="0",short='x',long)]
	/// Initial x-value
	x: u32,

	#[clap(default_value="0",short='y',long)]
	/// Initial y-value
	y: u32,

	#[clap(default_value="10",long)]
	/// Number of concurrent http-requests
	concurrent_requests: usize
}

// run a http GET-request (url), save the response content to file (path)
// return success or failure
fn run_request(url:&str, path:&str) -> Result<(),Box<dyn std::error::Error>>
{
	let client = reqwest::blocking::Client::builder().danger_accept_invalid_certs(true).build()?;
	let mut res = client.get(url).send()?;
	let mut file = std::fs::File::create(path)?;
	let _n = res.copy_to(&mut file)?;
	return Ok(());
}

fn main() {
	
	// parse command-line-arguments
	let args = Args::parse();
	//println!("{:?}", args);

	// dynamic array to store running threads
	let mut handles:Vec<thread::JoinHandle<()>> = Vec::new();

	// loop through zoom-levels
	for z in args.start_zoom..args.end_zoom+1 {
		
		// compute nr of tiles in this zoomlevel
		let n = u32::pow(2,z);

		// iterate through x-tiles
		for x in args.x..n {

			// make sure the directory z/x/ exists
			let directory = format!("{}/{}/{}", args.output_dir, z.to_string(), x.to_string());
			if !std::path::Path::new(&directory).is_dir() {
				std::fs::create_dir_all(&directory).unwrap();
			}

			// iterate through y-tiles
			for y in args.y..n {

				// inject x,y,z values into url
				let mut url = args.url
				.replace("{x}", x.to_string().as_str())
				.replace("{y}", y.to_string().as_str())
				.replace("{z}", z.to_string().as_str());

				// if applicable, inject bounds values
				if url.contains("{bounds}"){
					let lon_step = 360.0 / n as f32;
					let lon = (x as f32 * lon_step) - 180.0;
					let lat_step = 180.0 / n as f32;
					let lat = -(y as f32 * lat_step) + 90.0;
					url = url.replace("{bounds}", format!("({},{},{},{})", lat, lat-lat_step, lon, lon+lon_step).as_str());
				}

				// if there is more running threads - than arg.concurrent_threads
				while handles.len() > args.concurrent_requests {
					// take a little break, - waiting for threads to finish..
					thread::sleep(time::Duration::from_millis(10));

					// loop through the threads, and check if anyone has finished
					for i in 0..handles.len() {
						if let Some(h) = handles.get(i) {
							
							// thread is finished, remove it from the list
							if h.is_finished() {
								handles.remove(i);
							}
						}
					}
				}
				
				// spawn a new thread, - move the path and url variable into it
				let path = format!("{}/{}.png", directory, y.to_string());
				handles.push(thread::spawn(move||{

					if let Err(err) = run_request(url.as_str(), path.as_str()) {
						println!("Failed to save {}. Error: {}", url, err);
					}
				}));
			}
		}
	}

	// wait until all threads are done
	while !handles.is_empty() {
		thread::sleep(time::Duration::from_millis(100));
		for i in 0..handles.len() {
			if let Some(h) = handles.get(i) {
				if h.is_finished() {
					handles.remove(i);
				}
			}
		}
	}
}
