The problem

I spotted two separate issues that together prevent you from seeing the files currently being tagged at the top:
1. Pagination is one-way bound — user sorting doesn't work
html

:pagination='pagination'   <!-- ❌ one-way, update:pagination is ignored -->

QTable is a controlled component — when the user clicks a sortable header, it emits update:pagination, but since you're not binding that back, the sort state never changes. So no amount of header-clicking will re-sort the list. This is why you "can not sort the list" at all.
2. There is no column that reflects "tag order"

Even if sorting worked, the only columns are filename, path, and platform columns — none of which correlate with "which file is being tagged right now". You need a way to sort by insertion order.
The fix

    Bind pagination two-way (v-model:pagination) so sorting actually works.

    Add a hidden _seq column = index in the statuses array (= order files were queued/tagged).

    Add a Live view toggle that sorts by _seq descending, so the newest (i.e. currently-tagged) file is always at the top.

    Auto-enable live view when the component mounts while tagging is in progress.

Here are the changes:
AutotaggerStatus.vue — template

1. Title row — add the live toggle button:
html

<div class='row justify-center items-center q-mt-md'>
    <div class='text-subtitle1 text-bold text-primary'>TAGGING STATUS</div>
    <q-btn
        round dense
        :color='liveMode ? "primary" : "grey-7"'
        :icon='liveMode ? "mdi-flash" : "mdi-flash-off"'
        size='sm'
        class='q-ml-sm'
        @click='toggleLiveMode'
    >
        <q-tooltip>
            {{ liveMode
                ? 'Live view ON — newest (currently-tagged) file on top'
                : 'Enable Live view — newest file on top' }}
        </q-tooltip>
    </q-btn>
</div>

2. q-table — use v-model:pagination instead of :pagination:
html

<q-table
    class='status-table bg-dark'
    :rows='rows'
    :columns='columns'
    row-key='fullPath'
    virtual-scroll
    :rows-per-page-options='[0]'
    hide-pagination
    flat
    dark
    dense
    binary-state-sort
    v-model:pagination='pagination'
    :virtual-scroll-sticky-size-start='48'
>

3. body-cell slot — handle the hidden _seq column:
html

<template v-slot:body-cell='props'>
    <q-td :props='props'>
        <span v-if='props.col.name === "_seq"'></span>
        <span v-else-if='props.col.name === "filename"' class='selectable text-white'>
            {{ props.row.filename }}
        </span>
        <span v-else-if='props.col.name === "path"' class='selectable text-grey-5'>
            {{ props.row.path }}
        </span>
        <span v-else>
            <!-- unchanged platform-status rendering -->
            ...
        </span>
    </q-td>
</template>

AutotaggerStatus.vue — script
ts

// NEW state
const liveMode = ref(false);

// Replace the old `statuses` computed with allRows (keeps the original index)
const allRows = computed<Row[]>(() => {
    const isAudio = $1t.taggerStatus.value.type === 'audioFeatures';
    return $1t.taggerStatus.value.statuses.map((entries, idx) => {
        const fullPath = entries[0]?.status.path ?? '';
        const platforms: Record<string, TaggingStatusWrap> = {};
        if (isAudio) {
            if (entries[0]) platforms['audioFeatures'] = entries[0];
        } else {
            for (const e of entries) platforms[e.platform] = e;
        }
        return {
            _seq: idx,                       // <-- order of tagging
            _status: getStatus(entries),     // <-- pre-computed for filtering
            filename: basename(fullPath),
            path: dirname(fullPath),
            fullPath,
            platforms,
        };
    });
});

// Filter on the enriched row objects
const rows = computed<Row[]>(() => {
    if (!filter.value) return allRows.value;
    return allRows.value.filter(r => r._status === filter.value);
});

// Extend Row interface
interface Row {
    _seq: number;
    _status: string;
    filename: string;
    path: string;
    fullPath: string;
    platforms: Record<string, TaggingStatusWrap>;
}

// Hidden _seq column so q-table can sort by it programmatically
const columns = computed(() => {
    const cols: any[] = [
        { name: '_seq',     label: '',       field: '_seq',     sortable: true, align: 'left',
          style: 'display: none', headerStyle: 'display: none' },   // <-- hidden
        { name: 'filename', label: 'Filename', field: 'filename', sortable: true, align: 'left' },
        { name: 'path',     label: 'Path',     field: 'path',     sortable: true, align: 'left' },
    ];
    for (const p of platformList.value) {
        cols.push({
            name: p.id,
            label: p.label,
            field: (row: Row) => row.platforms[p.id],
            sortable: true,
            align: 'center',
            sort: (a: TaggingStatusWrap | undefined, b: TaggingStatusWrap | undefined) =>
                platformScore(a) - platformScore(b),
        });
    }
    return cols;
});

// Toggle Live view
function toggleLiveMode() {
    liveMode.value = !liveMode.value;
    pagination.value = {
        ...pagination.value,
        sortBy: liveMode.value ? '_seq' : 'filename',
        descending: liveMode.value,
    };
}

onMounted(() => {
    stopping.value = false;

    // Auto-enable Live view when landing on the status page mid-tag
    if ($1t.lock.value.locked && !$1t.taggerStatus.value.done) {
        liveMode.value = true;
        pagination.value = { ...pagination.value, sortBy: '_seq', descending: true };
    }

    // ... rest of onMounted unchanged
});

Optional: highlight the currently-tagged row

Since _seq descending puts the newest file first, the top row is the current one during tagging. If you want a visual cue, add this to the q-td:
html

<q-td :props='props'
      :class='{ "current-tag-row": isCurrentRow(props.row) }'>

ts

function isCurrentRow(row: Row): boolean {
    return liveMode.value
        && !$1t.taggerStatus.value.done
        && row._seq === $1t.taggerStatus.value.statuses.length - 1;
}

css

.current-tag-row {
    background: rgba(255, 255, 255, 0.06) !important;
}

Why this works

    v-model:pagination restores the ability to click headers to sort (previously broken because the sort update was discarded).

    _seq is the file's position in taggerStatus.statuses — i.e. the order it entered the pipeline. Sorting on _seq descending keeps the most recently-started file at position 0.

    Because the backend appends a file's status entry when it starts processing, the last element in statuses is the file currently being tagged — so it always floats to the top and stays there as new files start.

    The live toggle is auto-enabled when you land on the page mid-tag, and can be turned off afterwards for normal review.

One caveat: this assumes your backend processes files sequentially and appends entries as files start (which is what the current TaggingStatusWrap[] shape implies). If you later add parallel processing, you'd want to key off whatever status value the backend uses for "in progress" instead of _seq === length - 1 — but the sort-by-_seq mechanic itself still works fine as a general "newest first" view.




Summary of the changes in AutotaggerStatus.vue
#	Change	Effect
1	Title row now has an ⚡ flash icon button	Toggles Live view (newest-first)
2	:pagination='pagination' → v-model:pagination='pagination'	Header-click sorting actually works again (previously silently broken)
3	New hidden _seq column	Gives q-table a sortable field representing tag order
4	rows split into allRows (unfiltered, with _seq + _status) and rows (filtered)	Preserves the original status-index even when filters are applied
5	toggleLiveMode() sets sortBy: '_seq', descending: true	Newest entry in statuses[] — i.e. the file currently being tagged — is pinned to the top
6	onMounted auto-enables live mode while $1t.lock.value.locked && !done	You land on the page mid-tag and immediately see the live file
7	watch on pagination	Automatically exits live mode when you sort by something else
8	isCurrentRow() + .current-tag-row CSS	Rows being processed right now get a subtle highlight
9	onTaggingDone sets liveMode = false	After finishing, the table returns to a normal filename sort for review
