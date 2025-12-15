<script setup>
import { ref } from 'vue'
import JSZip from 'jszip'

const photos = ref([])
const watermark = ref(null)
const processing = ref(false)
const message = ref('')
const processTime = ref(null) // total process time from header
const startTime = ref(null) // timestamp when user started
const perPhotos = ref([]) // list of { name, status, duration }

const fotoInput = ref(null)
const watermarkInput = ref(null)

const handlePhotos = (files) => {
  photos.value = [...files]
}

const handleWatermark = (files) => {
  watermark.value = files[0]
}

const startProcess = async () => {
  if (photos.value.length === 0 || !watermark.value) {
    message.value = 'Harap unggah foto & watermark!'
    return
  }

  processing.value = true
  message.value = ''
  processTime.value = null

  // Set start time and per-photo initial status
  startTime.value = Date.now()
  perPhotos.value = photos.value.map((file, i) => ({
    name: file.name,
    index: i + 1,
    status: 'queued',
    duration: null,
  }))

  const form = new FormData()
  photos.value.forEach((file) => form.append('photos', file))
  form.append('watermark', watermark.value)

  try {
    const res = await fetch('http://127.0.0.1:3000/api/watermark', {
      method: 'POST',
      body: form,
    })

    // Ambil waktu proses dari header (total)
    const time = res.headers.get('x-process-time')
    if (time) processTime.value = parseFloat(time)

    // Ambil blob ZIP
    const blob = await res.blob()

    // Baca manifest.json dari ZIP menggunakan JSZip
    try {
      const zip = await JSZip.loadAsync(blob)
      const manifestFile = zip.file('manifest.json')
      if (manifestFile) {
        const manifestText = await manifestFile.async('string')
        try {
          const manifest = JSON.parse(manifestText)
          // manifest is expected to be array of {file, duration}
          manifest.forEach((item) => {
            // file name is watermarked_<index>.png
            const m = item.file.match(/watermarked_(\d+)\.png/)
            if (m) {
              const idx = parseInt(m[1], 10) - 1
              if (perPhotos.value[idx]) {
                perPhotos.value[idx].status = 'done'
                perPhotos.value[idx].duration = item.duration
              }
            }
          })
        } catch (e) {
          console.warn('Gagal parse manifest.json', e)
        }
      }
    } catch (e) {
      console.warn('Gagal baca ZIP manifest', e)
    }

    // Download ZIP file
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'hasil_watermark.zip'
    a.click()

    message.value = 'Berhasil! ZIP berhasil didownload.'
  } catch (err) {
    console.error(err)
    message.value = 'Gagal memproses foto.'
  }

  processing.value = false
}
</script>

<template>
  <div class="wrapper">
    <div class="card">
      <h1 class="title">Aplikasi Watermark Paralel</h1>

      <!-- FOTO -->
      <div
        class="dropzone"
        @click="fotoInput.click()"
        @dragover.prevent
        @drop.prevent="handlePhotos($event.dataTransfer.files)"
      >
        <p class="label">Tarik banyak foto ke sini atau klik</p>
        <p class="count">{{ photos.length }} foto dipilih</p>
        <input
          type="file"
          ref="fotoInput"
          multiple
          hidden
          @change="handlePhotos($event.target.files)"
        />
      </div>

      <!-- WATERMARK -->
      <div
        class="dropzone"
        @click="watermarkInput.click()"
        @dragover.prevent
        @drop.prevent="handleWatermark($event.dataTransfer.files)"
      >
        <p class="label">Tarik Watermark PNG ke sini</p>
        <p class="count">
          {{ watermark ? watermark.name : 'Belum dipilih' }}
        </p>
        <input
          type="file"
          ref="watermarkInput"
          accept="image/png"
          hidden
          @change="handleWatermark($event.target.files)"
        />
      </div>

      <button class="btn" :disabled="processing" @click="startProcess">
        {{ processing ? 'Memproses...' : 'Proses Foto' }}
      </button>

      <p class="message">{{ message }}</p>
      
      <div v-if="startTime" class="process-time">
        <span class="lightning">🕒</span>
        <span class="time-label">Waktu Mulai:</span>
        <span class="time-value">{{ new Date(startTime).toLocaleTimeString() }}</span>
      </div>

      <div v-if="perPhotos.length" style="margin-top:12px; text-align:left;">
        <h4 style="color:#ddd; margin:8px 0">Status Per Foto</h4>
        <ul style="list-style:none; padding:0; margin:0;">
          <li v-for="p in perPhotos" :key="p.index" style="display:flex; gap:8px; align-items:center; padding:6px 0; border-bottom:1px solid rgba(255,255,255,0.04)">
            <div style="width:28px; text-align:center; color:#aaa">#{{ p.index }}</div>
            <div style="flex:1; color:#fff">{{ p.name }}</div>
            <div style="width:140px; text-align:right; color:#9bd1ff">
              <template v-if="p.status === 'done'">{{ p.duration.toFixed(5) }} s</template>
              <template v-else>{{ p.status }}</template>
            </div>
          </li>
        </ul>
      </div>

      <div v-if="processTime !== null" class="process-time">
        <span class="lightning">⚡</span>
        <span class="time-label">Waktu Proses (total):</span>
        <span class="time-value">{{ processTime.toFixed(5) }} detik</span>
      </div>
    </div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}

html,
body,
#app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: #0d0d0d;
  font-family: 'Inter', sans-serif;
}

.wrapper {
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
}

.card {
  background: #1a1a1a;
  width: 380px;
  padding: 32px;
  border-radius: 14px;
  text-align: center;
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.6);
  animation: fadeIn 0.3s ease-out;
}

.title {
  font-size: 22px;
  font-weight: 600;
  color: #fff;
  margin-bottom: 26px;
}

.dropzone {
  border: 2px dashed #555;
  padding: 22px;
  border-radius: 10px;
  margin-bottom: 20px;
  cursor: pointer;
  transition: 0.2s;
}

.dropzone:hover {
  background: rgba(255, 255, 255, 0.05);
}

.label {
  color: #ddd;
  font-size: 14px;
}

.count {
  margin-top: 4px;
  font-size: 12px;
  color: #888;
}

.btn {
  width: 100%;
  padding: 14px;
  border-radius: 8px;
  border: none;
  background: #4da3ff;
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: 0.2s;
}

.btn:hover {
  background: #2b8cff;
}

.btn:disabled {
  background: #333;
  cursor: not-allowed;
}

.message {
  margin-top: 12px;
  color: #ff7f7f;
  font-size: 13px;
  min-height: 18px;
}

.process-time {
  margin-top: 16px;
  padding: 12px 16px;
  background: rgba(77, 163, 255, 0.1);
  border: 1px solid rgba(77, 163, 255, 0.3);
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

.lightning {
  font-size: 18px;
}

.time-label {
  color: #4da3ff;
  font-weight: 600;
}

.time-value {
  color: #fff;
  font-weight: 700;
  margin-left: auto;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
