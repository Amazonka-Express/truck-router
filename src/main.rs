use kd_tree::KdPoint;
use tonic::transport::Server;
use truck::{Coordinates, RouteNode, truck_router_server::TruckRouterServer};

pub mod truck {
    tonic::include_proto!("truck");
}

pub struct Router {
    ch: ch_router::ContractionHierarchy,
    kd_tree: kd_tree::KdTree<Node>,
    points: Vec<Coordinates>,
}

struct Node {
    longitude: f32,
    latitude: f32,
    router_id: u32,
}

impl KdPoint for Node {
    type Scalar = f32;
    type Dim = typenum::U2;

    fn at(&self, i: usize) -> Self::Scalar {
        match i {
            0 => self.longitude,
            1 => self.latitude,
            _ => panic!("Invalid index"),
        }
    }
}

impl KdPoint for Coordinates {
    type Scalar = f32;
    type Dim = typenum::U2;

    fn at(&self, i: usize) -> Self::Scalar {
        match i {
            0 => self.longitude,
            1 => self.latitude,
            _ => panic!("Invalid index"),
        }
    }
}

impl truck::truck_router_server::TruckRouter for Router {
    fn get_route<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<truck::RouteRequest>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<
                    Output = std::result::Result<tonic::Response<truck::Route>, tonic::Status>,
                > + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let mut path = vec![];

        for (idx, from_to) in request.into_inner().coordinates.windows(2).enumerate() {
            let from = from_to[0];
            let to = from_to[1];

            let from = self.kd_tree.nearest(&from).unwrap().item;
            let to = self.kd_tree.nearest(&to).unwrap().item;

            let route = self
                .ch
                .route(from.router_id, to.router_id)
                .unwrap()
                .path
                .into_iter()
                .map(|id| RouteNode {
                    coordinates: Some(Coordinates {
                        longitude: self.points[id as usize].longitude,
                        latitude: self.points[id as usize].latitude,
                    }),
                    stop_index: if id == from.router_id {
                        Some(idx as u32)
                    } else if id == to.router_id {
                        Some(idx as u32 + 1)
                    } else {
                        None
                    },
                });

            path.extend(route);
        }

        Box::pin(async move { Ok(tonic::Response::new(truck::Route { coordinates: path })) })
    }
}

fn load_points() -> Vec<Coordinates> {
    use std::{
        fs,
        io::{BufReader, Read},
    };

    let mut file = BufReader::new(fs::File::open("./nodes.bin").unwrap());

    let mut buf = [0u8; 4];
    let mut buf_8 = [0u8; 8];

    let mut nodes = vec![];

    file.read_exact(&mut buf).unwrap();
    let nodes_len = u32::from_le_bytes(buf);

    for _ in 0..nodes_len {
        file.read_exact(&mut buf).unwrap();
        file.read_exact(&mut buf).unwrap();

        file.read_exact(&mut buf_8).unwrap();
        let lat = f64::from_le_bytes(buf_8);
        file.read_exact(&mut buf_8).unwrap();
        let lon = f64::from_le_bytes(buf_8);

        nodes.push(Coordinates {
            longitude: lon as f32,
            latitude: lat as f32,
        });
    }

    nodes
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let points = load_points();

    let router = Router {
        ch: ch_router::ContractionHierarchy::load("./ch.bin").unwrap(),
        kd_tree: kd_tree::KdTree::build_by_ordered_float(
            points
                .iter()
                .enumerate()
                .map(|(idx, point)| Node {
                    longitude: point.longitude,
                    latitude: point.latitude,
                    router_id: idx as u32,
                })
                .collect(),
        ),
        points,
    };

    println!("Truck router listening on {}", addr);

    Server::builder()
        .add_service(TruckRouterServer::new(router))
        .serve(addr)
        .await?;

    Ok(())
}
