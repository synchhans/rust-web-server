use std::{env, str::FromStr, time::SystemTime};

use chrono::Utc;
use futures_util::StreamExt;
use mongodb::{bson::{doc, from_document, oid::ObjectId, DateTime}, error::Error, results::{InsertOneResult, UpdateResult}, Client, Collection};


use crate::models::{booking_model::{Booking, FullBooking}, dog_model::Dog, owner_model::Owner};

pub struct Database {
    booking: Collection<Booking>,
    dog: Collection<Dog>,
    owner: Collection<Owner>,
}

impl Database {
    pub async fn init() -> Self {
        let uri = match env::var("MONGO_URI") {
            Ok(v) => v.to_string(),
            Err(_) => "mongodb://localhost:27017/?directConnection=true".to_string(),
        };

        let client = Client::with_uri_str(uri).await.unwrap();
        let db = client.database("dog_walking");

        let booking: Collection<Booking> = db.collection("booking");
        let dog: Collection<Dog> = db.collection("dog");
        let owner: Collection<Owner> = db.collection("owner");

        Database {
            booking,
            dog,
            owner,
        }
    }

    pub async fn create_owner(&self, owner: Owner) -> Result<InsertOneResult, Error> {
        let result = self
        .owner
        .insert_one(owner)
        .await
        .ok()
        .expect("Error create owner");

        Ok(result)
    }
    pub async fn create_dog(&self, dog: Dog) -> Result<InsertOneResult, Error> {
        let result = self
        .dog
        .insert_one(dog)
        .await
        .ok()
        .expect("Error create dog");

        Ok(result)
    }
    pub async fn create_booking(&self, booking: Booking) -> Result<InsertOneResult, Error> {
        let result = self
        .booking
        .insert_one(booking)
        .await
        .ok()
        .expect("Error create booking");

        Ok(result)
    }

    pub async fn cancel_booking(&self, booking_id: &str) -> Result<UpdateResult, Error> {
        let result = self
        .booking
        .update_one(doc! {
            "_id": ObjectId::from_str(booking_id).expect("Failed to parse booking_id")
        }, doc! {
            "$set": doc! {
                "cancelled": true
            }
        })
        .await
        .ok()
        .expect("Error cancelling booking");
        
        Ok(result)
    }

    pub async fn get_bookings(&self) -> Result<Vec<FullBooking>,Error> {
        let now: SystemTime = Utc::now().into();

        let mut results = self
        .booking
        .aggregate(vec![
            doc! {
                "$match": {
                    "cancelled": false,
                    "start_time": {
                        "$gte": DateTime::from_system_time(now)
                    }
                }
            },
            doc! {
                "$lookup": doc! {
                    "from":"owner",
                    "localField": "owner",
                    "foreignField": "_id",
                    "as": "owner"
                }
            },
            doc! {
                "$unwind": doc! {
                    "path": "owner"
                }
            },
            doc! {
                "$lookup": doc! {
                    "from":"dog",
                    "localField": "owner._id",
                    "foreignField": "owner",
                    "as": "dogs"
                }
            },
        ])
        .await
        .ok()
        .expect("Error getting full bookings");

        let mut bookings: Vec<FullBooking> = Vec::new();

        while let Some(result) = results.next().await {
            match result {
                Ok(doc) => {
                    let booking: FullBooking = from_document(doc).expect("Error converting document to full boking");
                    bookings.push(booking);
                },
                Err(err) => panic!("Error getting booking: {}",err),
            }
        }

        Ok(bookings)
    }

}