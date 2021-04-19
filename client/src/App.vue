<template>
<div>
  <q-layout view="hHh lpR fFf" class='bg-background'>

    <!-- Header -->
    <q-header class="bg-darker text-white" height-hint="98">
      <q-toolbar>
        <q-toolbar-title class='topbar'>
          <img src="./assets/icon.png" height='42' @click='home' class='logo' :class='{spin: $1t.lock.locked}'>
          <img src="./assets/logo-text.png" height='42' @click='home' class='logo'>
        </q-toolbar-title>
        <!-- Settings -->
        <q-btn flat round dense icon='mdi-cog' @click='settings = true'></q-btn>
        
      </q-toolbar>

      <q-tabs align="left">
        <q-route-tab :disable='$1t.lock.locked' to="/" class='text-weight-bolder' @click='hideSide'><q-icon name='mdi-home' size='sm'></q-icon></q-route-tab>
        <q-route-tab :disable='$1t.lock.locked' to="/autotagger" class='text-weight-bolder' @click='hideSide'>Auto tag</q-route-tab>
        <q-route-tab :disable='$1t.lock.locked' to="/audiofeatures" class='text-weight-bolder' @click='audioFeatures'>Audio features</q-route-tab>
        <q-route-tab :disable='$1t.lock.locked' to="/quicktag" class='text-weight-bolder' @click='showSide'>Quick Tag</q-route-tab>
      </q-tabs>
    </q-header>

    <HelpButton></HelpButton>

    <!-- Drawers -->
    <q-drawer :breakpoint='1000' v-model="left" side="left" :width='200'>
      <QuickTagLeft></QuickTagLeft>
    </q-drawer>
    <q-drawer :breakpoint='1000' v-model="right" side="right" :width='200'>
      <QuickTagRight></QuickTagRight>
    </q-drawer>

    <!-- Content -->
    <q-page-container class='content' ref='contentContainer'>
      <transition name='fade'>
        <router-view />
      </transition>
    </q-page-container>

    <!-- Footer -->
    <q-footer reveal class="bg-darker text-white" v-if='footer'>
      <QuickTagGenreBar v-if='$1t.quickTag.track'></QuickTagGenreBar>
      <q-toolbar>
        <div class='row'>
          <!-- Play button -->
          <q-btn 
            round 
            flat
            icon='mdi-play' 
            class='q-mr-sm' 
            :ripple='false' 
            v-if='!$1t.player.playing'
            @click='$1t.play()'
          ></q-btn>
          <!-- Pause -->
          <q-btn 
            round 
            flat
            icon='mdi-pause' 
            class='q-mr-sm' 
            :ripple='false' 
            v-if='$1t.player.playing'
            @click='$1t.pause()'
          ></q-btn>
        </div>
        <Waveform></Waveform>
        <!-- Volume -->
        <div class='volume-container'>
          <q-slider
            v-model='$1t.player.volume'
            :min='0.00'
            :max='1.00'
            :step='0.01'
            @input='$1t.setVolume($event)'
            @change='$1t.saveSettings()'
          ></q-slider>
        </div>
      </q-toolbar>
    </q-footer>

  </q-layout>

  <!-- Settings -->
  <Settings v-model='settings' @close='settings = false'></Settings>

  <!-- Min size dialog -->
  <q-dialog v-model='sizeDialog' persistent>
    <q-card>
      <q-card-section>
        <div class='text-h6'>Warning</div>
      </q-card-section>
      <q-card-section>
        One Tagger requires atleast 1024x550 window size. Please resize to continue.
      </q-card-section>
    </q-card>
  </q-dialog>

</div>
</template>

<script>
import Waveform from './components/Waveform.vue';
import QuickTagLeft from './components/QuickTagLeft';
import Settings from './components/Settings';
import QuickTagGenreBar from './components/QuickTagGenreBar';
import QuickTagRight from './components/QuickTagRight';
import HelpButton from './components/HelpButton';

export default {
  name: "App",
  components: {Waveform, QuickTagLeft, Settings, QuickTagGenreBar, QuickTagRight, HelpButton},
  data() {
    return {
      left: false,
      right: false,
      footer: false,
      settings: false,
      sizeDialog: false
    };
  },
  methods: {
    //Hide/Show footer and drawer
    hideSide() {
      this.left = false;
      this.right = false;
      this.footer = false;
    },
    showSide() {
      this.left = true;
      this.right = true;
      this.footer = true;
    },
    //Navigate to homepage
    home() {
      if (!this.$1t.lock.locked) {
        this.hideSide();
        this.$router.push("/");
      }
    },
    //Navigate to audio features
    audioFeatures() {
      if (!this.$1t.lock.locked) {
        this.hideSide();
        this.$router.push('/audiofeatures');
      }
    }
  },
  mounted() {
    this.$q.dark.set(true);

    //Handle resize to apply min height/width
    window.addEventListener('resize', () => {
      if (window.innerHeight < 550 || window.innerWidth < 1024) {
        this.sizeDialog = true;
      } else {
        this.sizeDialog = false;
      }
    });
  },
  watch: {
    //Dont show scrollbar while transition
    $route() {
      this.$refs.contentContainer.$el.style.overflowY = "hidden";
    }
  },
  updated() {
    //Show again scrollbar after transition
    setTimeout(() => {
      this.$refs.contentContainer.$el.style.overflowY = "auto";
    }, 250);
  }
};
</script>

<style>
.content {
  overflow-y: auto;
  height: calc(100vh);
  min-height: 100vh;
}
.logo {
  cursor: pointer;
}
.topbar {
  padding-top: 12px !important;
}
.volume-container {
  padding-left: 40px;
  width: 18% !important;
}

.fade-enter-active, .fade-leave-active {
  transition-property: opacity;
  transition-duration: .25s;
}
.fade-enter-active {
  transition-delay: .25s;
}
.fade-enter, .fade-leave-active {
  opacity: 0
}

@keyframes rotation {
  from {transform: rotate(0deg);}
  to {transform: rotate(360deg);}
}
.spin {
  animation: rotation 2s infinite linear;
}
</style>