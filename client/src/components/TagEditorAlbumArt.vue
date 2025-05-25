<template>
  <!-- Image -->
  <div
    @dragover.prevent="drag = true"
    @dragleave.prevent="drag = false"
    @drop.prevent="drop"
  >
    <q-img
      :src="image.data"
      class="albumart clickable"
      @click="emit('click')"
      :class="{ 'albumart-darker': drag }"
    >
      <div
        v-show="drag"
        class="text-h6 full-height row justify-center items-center text-center"
      >
        Drop the image to replace
      </div>
    </q-img>
  </div>

  <!-- Meta -->
  <div class="q-pt-sm q-mb-md">
    <div class="text-caption">{{ image.kind }}</div>
    <div class="text-caption">{{ image.description }}</div>
    <div class="text-subtitle3 text-grey-6 monospace">
      {{ image.mime }} {{ image.width }}x{{ image.height }}
    </div>
    <q-btn
      dense
      push
      color="red"
      class="rounded-borders q-px-md q-mt-sm text-weight-medium"
      @click="emit('remove')"
      >Remove</q-btn
    >
  </div>
</template>

<script lang="ts" setup>
import { PropType, ref } from "vue";

interface TagEditorImage {
  mime: string;
  data?: string;
  kind?: string;
  description: string;
  width: number;
  height: number;
}

const { image } = defineProps({
  image: { type: Object as PropType<TagEditorImage>, required: true },
});
const emit = defineEmits(["click", "remove", "replace"]);

const drag = ref(false);

/// File dropped, replace

function drop(e: DragEvent) {
  // Get file
  let files = e.dataTransfer!.files;
  if (files.length !== 1) return;
  let file = files[0];
  if (!file.type.includes("image/")) return;

  // Read
  let reader = new FileReader();
  reader.onload = (f) => {
    replaceArt(f.target!.result as string);
  };
  reader.readAsDataURL(file);
  drag.value = false;
}

async function replaceArt(data: string) {
  // Load width/height
  let wh: [number, number] = await new Promise((res) => {
    let i = new Image();
    i.onload = function () {
      res([i.width, i.height]);
    };
    i.src = data;
  });

  // Output image
  let outImage: any = {
    data: data.substring(data.indexOf("base64,") + 7).trim(),
    mime: data.substring(5, data.indexOf(";")),
    description: image.description ?? "",
    kind: image.kind,
    width: wh[0],
    height: wh[1],
  };
  emit("replace", outImage);
}
</script>

<style lang="scss" scoped>
.albumart-darker {
  opacity: 0.6;
}
</style>
