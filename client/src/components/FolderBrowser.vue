<template>
  <div class="folder-browser">
    <q-card class="q-pa-md">
      <div class="text-h6">Select folder</div>

      <!-- Tree view -->
      <div class="q-my-md folder-list" ref="folderList">
        <q-tree
          dense
          :nodes="folders"
          @lazy-load="lazyLoad"
          node-key="path"
          v-if="baseLoaded"
          default-expand-all
        >
          <template v-slot:default-header="prop">
            <div @click="path = prop.node.path" class="row clickable">
              <q-icon
                :class="{ 'text-primary': prop.node.path == path }"
                name="mdi-folder"
                size="xs"
                class="q-pr-sm"
              ></q-icon>
              <div
                :class="{ 'text-bold text-primary': prop.node.path == path }"
              >
                {{ prop.node.label }}
              </div>
            </div>
          </template>
        </q-tree>
      </div>

      <!-- Actions -->
      <div>
        <q-input filled dense label="Path" v-model="path"></q-input>
        <div class="row justify-end q-pt-md">
          <q-btn flat color="red" @click="cancel">Cancel</q-btn>
          <q-btn class="q-ml-md" flat color="primary" @click="save">OK</q-btn>
        </div>
      </div>
    </q-card>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from "vue";
import { get1t } from "../scripts/onetagger.js";

const { base } = defineProps({
  base: { type: String, default: "/" },
});
const $1t = get1t();
const path = ref("/");
const folders = ref<any[]>([]);
const baseLoaded = ref(false);
let onResolve: any = undefined;

// Tree lazy loading
function lazyLoad({ node, done }: { node: any; done: any }) {
  path.value = node.parent;
  onResolve = done;
  $1t.send("folderBrowser", {
    path: path.value,
    child: node.label,
    base: false,
  });
}

// Convert entry into a tree node
function convEntry(entry: any) {
  if (entry.children)
    entry.children = entry.children.map((e: any) => {
      let parts = e.path.replace(/\\/g, "/").split("/");
      // Recurse
      if (e.children) {
        e = convEntry(e);
      }
      return {
        label: parts[parts.length - 1],
        lazy: e.children ? false : true,
        parent: entry.path,
        path: e.path,
        children: e.children,
      };
    });
  return entry;
}

// Find scroll offset of a path
function findScrollOffset(entry: any, parts: any[], prev = 0): number {
  for (let i = 0; i < (entry.children ?? []).length; i++) {
    if (entry.children[i].label == parts[0]) {
      parts.splice(0, 1);
      return findScrollOffset(entry.children[i], parts, prev + i);
    }
  }
  return prev;
}

// Split path to parts
function pathParts(path: string) {
  let parts = path.replace(/\\/g, "/").split("/");
  if (parts.length == 0) return [];
  if (parts[0] == "") {
    parts.splice(0, 1);
  }
  return parts;
}

// Close
function cancel() {
  $1t.folderBrowser.value.open = false;
}

// Save
function save() {
  $1t.onBrowse({
    context: $1t.folderBrowser.value.context,
    path: path.value,
    action: "browse",
  });
  $1t.folderBrowser.value.open = false;
}

const folderList = ref<any>();
onMounted(() => {
  // Register events
  $1t.onFolderBrowserEvent = (json) => {
    switch (json.action) {
      case "folderBrowser":
        // Base folder structure
        if (json.base) {
          var entry = convEntry(json.entry);
          folders.value = entry.children;
          path.value = json.path;
          baseLoaded.value = true;

          // Scroll
          setTimeout(() => {
            let offset = findScrollOffset(entry, pathParts(path.value));
            folderList.value!.scrollTo({
              top: offset * 23,
              behavior: "smooth",
            });
          }, 64);

          break;
        }

        // Structure
        var f = convEntry(json.entry).children;

        // Resolve lazy load
        if (onResolve) {
          onResolve(f);
          onResolve = undefined;
        } else {
          folders.value = f;
          baseLoaded.value = true;
        }

        path.value = json.path;
        break;
    }
  };

  // Load base
  if (base) {
    $1t.send("folderBrowser", { path: base, child: "", base: true });
  } else {
    $1t.send("folderBrowser", { path: path.value, child: "", base: false });
  }
});
</script>

<style lang="scss">
.folder-browser {
  width: 500px;
  height: 590px;
}
.folder-list {
  overflow-y: scroll;
  height: 400px;
}
</style>
