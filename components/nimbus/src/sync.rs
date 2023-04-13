/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

 //! A module that defines what it means for nimbus to have
 //! a syncable store.
 //!
 //!
 //!
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{sync::{Arc, Weak}, collections::HashMap};
use sync15::{engine::{SyncEngine, SyncEngineId, OutgoingChangeset, CollectionRequest, EngineSyncAssociation, CollSyncIds}, bso::{OutgoingBso, IncomingBso, OutgoingEnvelope}, telemetry, Guid, ClientData, RemoteClient, DeviceType};
use crate::{persistence::StoreId, enrollment::ExperimentEnrollment};

use super::persistence::Database;
pub(crate) static GLOBAL_SYNCID_META_KEY: &str = "global_sync_id";
pub(crate) static COLLECTION_SYNCID_META_KEY: &str = "nimbus_sync_id";
pub(crate) static REMOTE_CLIENTS_KEY: &str = "remote_clients";


 pub struct NimbusEngine {
    pub sync_impl: Mutex<NimbusSyncImpl>,
 }

 impl NimbusEngine {
    fn new(store: Arc<Database>) -> Self {
        Self {
            sync_impl: Mutex::new(NimbusSyncImpl::new(store))
        }
    }
 }


#[derive(Serialize, Deserialize, Default, Debug)]
struct EnrollmentsRecord {
    id: Guid,
    enrollments: Vec<ExperimentEnrollment>
}


 pub struct NimbusSyncImpl {
    pub(super) store: Arc<Database>,
    pub(super) local_id: String
 }

 impl NimbusSyncImpl {
    fn new(store: Arc<Database>) -> Self {
        Self {
            store,
            local_id: Default::default()
        }
    }

    fn prepare_for_sync(&mut self, client_data: ClientData) -> crate::Result<()> {
        let store = self.store.get_store(StoreId::Meta);
        let mut writer = self.store.write()?;
        store.put(&mut writer, REMOTE_CLIENTS_KEY, &client_data.recent_clients)?;
        writer.commit()?;
        self.local_id = client_data.local_client_id;
        Ok(())
    }

    fn apply_incoming(&mut self,
        inbound: Vec<IncomingBso>,
        _telem: &mut telemetry::Engine
    ) -> crate::Result<Vec<OutgoingBso>> {
        let mut remote_enrollments = Vec::with_capacity(inbound.len());

        let remote_clients: HashMap<String, RemoteClient> = {
            let store = self.store.get_store(StoreId::Meta);
            let reader = self.store.read()?;

            store.get(&reader, REMOTE_CLIENTS_KEY)?.unwrap_or_default()
        };
        for incoming in inbound {
            if incoming.envelope.id == self.local_id {
                // That's our own record, ignore it.
                continue;
            }

            let record = match incoming.into_content::<EnrollmentRecord>().content() {
                Some(record) => record,
                None => {
                    // Invalid record or a "tombstone" which tabs don't have.
                    log::warn!("Ignoring incoming invalid tab");
                    continue;
                }
            };
            remote_enrollments.push(record)
        }

        // We want to keep the mutex for as short as possible
        let local_enrollments: Vec<ExperimentEnrollment> = {
            // In desktop we might end up here with zero records when doing a quick-write, in
            // which case we don't want to wipe the DB.
            if !remote_enrollments.is_empty() {
                let store = self.store.get_store(StoreId::RemoteEnrollments);
                let mut writer = self.store.write()?;
                for enrollment in remote_enrollments {
                    store.put(&mut writer, &enrollment.id, &enrollment)?;
                }
                writer.commit()?;
            }
            let reader = self.store.read()?;
            let store = self.store.get_store(StoreId::Enrollments);
            store.collect_all(&reader)?
        };

        let outgoing = {
            let (client_name, _) = remote_clients
                .get(&self.local_id)
                .map(|client| (client.device_name.clone(), client.device_type))
                .unwrap_or_else(|| (String::new(), DeviceType::Unknown));
            let local_record = EnrollmentRecord {
                id: self.local_id.clone(),
                client_name: client_name,
                enrollments: local_enrollments,
            };
            log::trace!("outgoing {:?}", local_record);
            let envelope = OutgoingEnvelope {
                id: self.local_id.clone().into(),
                ..Default::default()
            };
            vec![OutgoingBso::from_content(
                envelope,
                local_record,
            )?]
        };

        Ok(outgoing)
    }

    pub fn get_sync_assoc(&self) -> crate::Result<EngineSyncAssociation> {
        let store = self.store.get_store(StoreId::Meta);
        let reader = self.store.read()?;
        let global = store.get(&reader, GLOBAL_SYNCID_META_KEY)?;
        let coll = store.get(&reader, COLLECTION_SYNCID_META_KEY)?;
        Ok(if let (Some(global), Some(coll)) = (global, coll) {
            EngineSyncAssociation::Connected(CollSyncIds {
                global: Guid::from_string(global),
                coll: Guid::from_string(coll),
            })
        } else {
            EngineSyncAssociation::Disconnected
        })
    }


    pub fn reset(&mut self, assoc: &EngineSyncAssociation) -> crate::Result<()> {
        let store = self.store.get_store(StoreId::Meta);
        let mut writer = self.store.write()?;
        match assoc {
            EngineSyncAssociation::Disconnected => {
                store.delete(&mut writer, GLOBAL_SYNCID_META_KEY)?;
                store.delete(&mut writer, COLLECTION_SYNCID_META_KEY)?;
            }
            EngineSyncAssociation::Connected(ids) => {
                store.put(&mut writer, GLOBAL_SYNCID_META_KEY, &ids.global.to_string())?;
                store.put(&mut writer, COLLECTION_SYNCID_META_KEY, &ids.coll.to_string())?;
            }
        };
        writer.commit()?;
        Ok(())
    }
}
// Our "sync manager" will use whatever is stashed here.
lazy_static::lazy_static! {
    // Mutex: just taken long enough to update the inner stuff
    static ref STORE_FOR_MANAGER: Mutex<Weak<Database>> = Mutex::new(Weak::new());
}



/// Called by the sync manager to get a sync engine via the store previously
/// registered with the sync manager.
pub fn get_registered_sync_engine(engine_id: &SyncEngineId) -> Option<Box<dyn SyncEngine>> {
    let weak = STORE_FOR_MANAGER.lock();
    match weak.upgrade() {
        None => None,
        Some(store) => match engine_id {
            SyncEngineId::Nimbus => Some(Box::new(NimbusEngine::new(Arc::clone(&store)))),
            // panicing here seems reasonable - it's a static error if this
            // it hit, not something that runtime conditions can influence.
            _ => unreachable!("can't provide unknown engine: {}", engine_id),
        },
    }
}

pub fn register_with_sync_manager(store: Arc<Database>) {
    let mut state = STORE_FOR_MANAGER.lock();
    *state = Arc::downgrade(&store);
}

 impl SyncEngine for NimbusEngine {
    fn collection_name(&self) -> sync15::CollectionName {
        "nimbus".into()
    }

    fn prepare_for_sync(&self, get_client_data: &dyn Fn() -> ClientData) -> anyhow::Result<()> {
        Ok(self
            .sync_impl
            .lock()
            .prepare_for_sync(get_client_data())?)
    }

    fn apply_incoming(
        &self,
        inbound: Vec<sync15::engine::IncomingChangeset>,
        telem: &mut sync15::telemetry::Engine,
    ) -> anyhow::Result<sync15::engine::OutgoingChangeset> {
        assert_eq!(inbound.len(), 1, "only requested one set of records");
        let inbound = inbound.into_iter().next().unwrap();
        let outgoing_records = self
            .sync_impl
            .lock()
            .apply_incoming(inbound.changes, telem)?;

        Ok(OutgoingChangeset::new("nimbus".into(), outgoing_records))
    }

    fn sync_finished(
        &self,
        _new_timestamp: sync15::ServerTimestamp,
        _records_synced: Vec<sync15::Guid>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_collection_requests(
        &self,
        _server_timestamp: sync15::ServerTimestamp,
    ) -> anyhow::Result<Vec<sync15::engine::CollectionRequest>> {
        Ok(vec![CollectionRequest::new("nimbus".into())
        .full()])
    }

    fn get_sync_assoc(&self) -> anyhow::Result<sync15::engine::EngineSyncAssociation> {
        Ok(self.sync_impl.lock().get_sync_assoc()?)
    }

    fn reset(&self, assoc: &sync15::engine::EngineSyncAssociation) -> anyhow::Result<()> {
        Ok(self.sync_impl.lock().reset(assoc)?)
    }

    fn wipe(&self) -> anyhow::Result<()> {
        self.reset(&EngineSyncAssociation::Disconnected)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnrollmentRecord {
    id: String,
    client_name: String,
    enrollments: Vec<ExperimentEnrollment>
}