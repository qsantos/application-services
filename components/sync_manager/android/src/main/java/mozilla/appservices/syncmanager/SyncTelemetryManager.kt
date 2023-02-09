package mozilla.appservices.syncmanager


/**
 * Import some private Glean types, so that we can use them in type declarations.
 *
 * I do not like importing these private classes, but I do like the nice generic
 * code they allow me to write! By agreement with the Glean team, we must not
 * instantiate anything from these classes, and it's on us to fix any bustage
 * on version updates.
 */
import org.mozilla.appservices.syncmanager.GleanMetrics.*
import org.mozilla.appservices.syncmanager.GleanMetrics.BookmarksSync
import org.mozilla.appservices.syncmanager.GleanMetrics.HistorySync
import org.mozilla.appservices.syncmanager.GleanMetrics.LoginsSync
import org.mozilla.appservices.syncmanager.GleanMetrics.Pings
import java.util.*

public class SyncTelemetryManagerImpl: SyncTelemetryManager {

    public fun registerWithSyncManager(syncManager: SyncManager) {
        syncManager.registerTelemetryManager(this)
    }

    override fun submitSyncPing() {
        Pings.sync.submit()
    }

    override fun recordStartTime(engine: SyncEngineId, startedAt: ULong) {
        when(engine) {
            SyncEngineId.HISTORY -> HistorySync.startedAt.set(Date(startedAt.toLong()))
            SyncEngineId.BOOKMARKS -> BookmarksSync.startedAt.set(Date(startedAt.toLong()))
            SyncEngineId.PASSWORDS -> LoginsSync.startedAt.set(Date(startedAt.toLong()))
            SyncEngineId.CREDIT_CARDS -> CreditcardsSync.startedAt.set(Date(startedAt.toLong()))
            SyncEngineId.ADDRESSES -> AddressesSync.startedAt.set(Date(startedAt.toLong()))
            SyncEngineId.TABS -> TabsSync.startedAt.set(Date(startedAt.toLong()))
        }
    }

    override fun recordEndTime(engine: SyncEngineId, endedAt: ULong) {
        when(engine) {
            SyncEngineId.HISTORY -> HistorySync.finishedAt.set(Date(endedAt.toLong()))
            SyncEngineId.BOOKMARKS -> BookmarksSync.finishedAt.set(Date(endedAt.toLong()))
            SyncEngineId.PASSWORDS -> LoginsSync.finishedAt.set(Date(endedAt.toLong()))
            SyncEngineId.CREDIT_CARDS -> CreditcardsSync.finishedAt.set(Date(endedAt.toLong()))
            SyncEngineId.ADDRESSES -> AddressesSync.finishedAt.set(Date(endedAt.toLong()))
            SyncEngineId.TABS -> TabsSync.finishedAt.set(Date(endedAt.toLong()))
        }
    }

    override fun recordIncomingRecords(engine: SyncEngineId, incomingRecords: EngineIncoming) {
        val applied = incomingRecords.applied.toInt()
        val failed = incomingRecords.failed.toInt()
        val reconciled = incomingRecords.reconciled.toInt()

        when(engine) {
            SyncEngineId.HISTORY -> {
                HistorySync.incoming["applied"].add(applied)
                HistorySync.incoming["failed_to_apply"].add(failed)
                HistorySync.incoming["reconciled"].add(reconciled)
            }
            SyncEngineId.BOOKMARKS -> {
                BookmarksSync.incoming["applied"].add(applied)
                BookmarksSync.incoming["failed_to_apply"].add(failed)
                BookmarksSync.incoming["reconciled"].add(reconciled)
            }
            SyncEngineId.PASSWORDS -> {
                LoginsSync.incoming["applied"].add(applied)
                LoginsSync.incoming["failed_to_apply"].add(failed)
                LoginsSync.incoming["reconciled"].add(reconciled)
            }
            SyncEngineId.CREDIT_CARDS -> {
                CreditcardsSync.incoming["applied"].add(applied)
                CreditcardsSync.incoming["failed_to_apply"].add(failed)
                CreditcardsSync.incoming["reconciled"].add(reconciled)
            }
            SyncEngineId.ADDRESSES -> {
                AddressesSync.incoming["applied"].add(applied)
                AddressesSync.incoming["failed_to_apply"].add(failed)
                AddressesSync.incoming["reconciled"].add(reconciled)
            }
            SyncEngineId.TABS -> {
                TabsSync.incoming["applied"].add(applied)
                TabsSync.incoming["failed_to_apply"].add(failed)
                TabsSync.incoming["reconciled"].add(reconciled)
            }
        }
    }

    override fun recordOutgoingRecords(engine: SyncEngineId, outgoingRecords: EngineOutgoing) {
        val failedNum = outgoingRecords.failed.toInt()
        val succeededNum = outgoingRecords.sent.toInt()
        when(engine) {
            SyncEngineId.HISTORY -> {
                HistorySync.outgoing["uploaded"].add(succeededNum)
                HistorySync.outgoing["failed_to_upload"].add(failedNum)
            }
            SyncEngineId.BOOKMARKS -> {
                BookmarksSync.outgoing["uploaded"].add(succeededNum)
                BookmarksSync.outgoing["failed_to_upload"].add(failedNum)
            }
            SyncEngineId.PASSWORDS -> {
                LoginsSync.outgoing["uploaded"].add(succeededNum)
                LoginsSync.outgoing["failed_to_upload"].add(failedNum)
            }
            SyncEngineId.CREDIT_CARDS -> {
                CreditcardsSync.outgoing["uploaded"].add(succeededNum)
                CreditcardsSync.outgoing["failed_to_upload"].add(failedNum)
            }
            SyncEngineId.ADDRESSES -> {
                AddressesSync.outgoing["uploaded"].add(succeededNum)
                AddressesSync.outgoing["failed_to_upload"].add(failedNum)
            }
            SyncEngineId.TABS -> {
                TabsSync.outgoing["uploaded"].add(succeededNum)
                TabsSync.outgoing["failed_to_upload"].add(failedNum)
            }
        }

    }

    override fun recordOutgoingBatches(engine: SyncEngineId, batches: ULong) {
        val batchesInt = batches.toInt()
        when(engine) {
            SyncEngineId.HISTORY -> HistorySync.outgoingBatches.add(batchesInt)
            SyncEngineId.BOOKMARKS -> BookmarksSync.outgoingBatches.add(batchesInt)
            SyncEngineId.PASSWORDS -> LoginsSync.outgoingBatches.add(batchesInt)
            SyncEngineId.CREDIT_CARDS -> CreditcardsSync.outgoingBatches.add(batchesInt)
            SyncEngineId.ADDRESSES -> AddressesSync.outgoingBatches.add(batchesInt)
            SyncEngineId.TABS -> TabsSync.outgoingBatches.add(batchesInt)
        }
    }

    override fun recordFailureReason(
        engine: SyncEngineId,
        label: FailureReasonLabel,
        reason: String
    ) {
        val labelName = when (label) {
            FailureReasonLabel.AUTH -> "auth"
            FailureReasonLabel.OTHER -> "other"
            FailureReasonLabel.UNEXPECTED -> "unexpected"
        }
        when(engine) {
            SyncEngineId.HISTORY -> HistorySync.failureReason[labelName].set(reason)
            SyncEngineId.BOOKMARKS -> BookmarksSync.failureReason[labelName].set(reason)
            SyncEngineId.PASSWORDS -> LoginsSync.failureReason[labelName].set(reason)
            SyncEngineId.CREDIT_CARDS -> CreditcardsSync.failureReason[labelName].set(reason)
            SyncEngineId.ADDRESSES -> AddressesSync.failureReason[labelName].set(reason)
            SyncEngineId.TABS -> TabsSync.failureReason[labelName].set(reason)
        }
    }

    override fun submitSyncEnginePing(engine: SyncEngineId) {
        when(engine) {
            SyncEngineId.HISTORY -> Pings.historySync.submit()
            SyncEngineId.BOOKMARKS -> Pings.bookmarksSync.submit()
            SyncEngineId.PASSWORDS -> Pings.loginsSync.submit()
            SyncEngineId.CREDIT_CARDS -> Pings.creditcardsSync.submit()
            SyncEngineId.ADDRESSES -> Pings.addressesSync.submit()
            SyncEngineId.TABS -> Pings.tabsSync.submit()
        }
    }
}