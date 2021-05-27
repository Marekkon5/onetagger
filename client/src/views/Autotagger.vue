<template>
<div>

    <q-stepper 
        v-model='step' 
        header-nav 
        color='primary' 
        animated 
        alternative-labels
        flat 
        class='bg-dark-page'
        v-if='!$1t.settings.autoTaggerSinglePage'>

        <!-- Platforms -->
        <q-step 
            :name='0' 
            title='Select Platforms' 
            :done='step > 0 && $1t.config.platforms.length > 0'
            icon='mdi-web'
            :error='step > 0 && $1t.config.platforms.length == 0'
            class='text-center step'>

            <div class='text-h5 q-mt-md text-grey-4'>Select platforms</div>
            <div class='text-subtitle1 q-mt-xs text-grey-6'>Use the checkbox to enable/disable, drag and drop to reorder fallback</div>
            <AutotaggerPlatforms></AutotaggerPlatforms>
            
            <q-stepper-navigation>
                <q-btn @click="step+=1" color="primary" label="Next" class='text-black'/>
            </q-stepper-navigation>
        </q-step>

        <!-- Tags -->
        <q-step
            :name='1'
            title='Path & Tags'
            :done='$1t.config.path != null && $1t.config.path.trim().length != 0 && step > 1'
            icon='mdi-label-multiple'
            :error='!$1t.config.path && step > 1'
            class='text-center step'>

            <AutotaggerTags></AutotaggerTags>
            <q-stepper-navigation>
                <q-btn v-if='$1t.config.path' @click="step+=1" color="primary" label="Next" class='q-mt-sm text-black'/>
            </q-stepper-navigation>
        </q-step>

        <!-- Platform Specific -->
        <q-step
            :name='2'
            title='Platform Specific Settings'
            :done='step > 2'
            icon='mdi-tune'
            class='text-center step'>

            <AutotaggerPlatformSpecific></AutotaggerPlatformSpecific>
            <q-stepper-navigation>
                <q-btn @click="step+=1" color="primary" label="Next" class='text-black'/>
            </q-stepper-navigation>
        </q-step>

        <!-- Advanced -->
        <q-step
            :name='3'
            title='Advanced'
            :done='step > 3'
            icon='mdi-cog'
            class='text-center step'>

            <div class='text-h5 q-my-md text-grey-4'>Advanced</div>
            <AutotaggerAdvanced></AutotaggerAdvanced>
            <q-btn 
                v-if='canStart' 
                size='md'
                color='primary'
                class='q-my-md text-black'
                @click='startTagging'
            >Start</q-btn>
        </q-step>

    </q-stepper>

    <!-- Single page -->
    <div v-if='$1t.settings.autoTaggerSinglePage' class='text-center'>
        <!-- Platforms -->
        <div class='text-h5 q-mt-md text-grey-4'>Select platforms</div>
        <div class='text-subtitle1 q-mt-xs text-grey-6'>Use the checkbox to enable/disable, drag and drop to reorder fallback</div>
        <AutotaggerPlatforms dense></AutotaggerPlatforms>
        <AutotaggerTags></AutotaggerTags>
        <AutotaggerPlatformSpecific></AutotaggerPlatformSpecific>
        <AutotaggerAdvanced></AutotaggerAdvanced>
        <q-btn 
            v-if='canStart' 
            size='md'
            color='primary'
            class='q-my-md text-black'
            @click='startTagging'
        >Start</q-btn>
    </div>

</div>
</template>

<script>
import AutotaggerPlatforms from '../components/AutotaggerPlatforms';
import AutotaggerTags from '../components/AutotaggerTags';
import AutotaggerPlatformSpecific from '../components/AutotaggerPlatformSpecific';
import AutotaggerAdvanced from '../components/AutotaggerAdvanced';

export default {
    name: 'Autotagger',
    components: {AutotaggerPlatforms, AutotaggerTags, AutotaggerPlatformSpecific, AutotaggerAdvanced},
    data() {
        return {
            step: 0
        }
    },
    methods: {
        startTagging() {
            this.$1t.saveSettings();
            this.$1t.send('startTagging', {config: this.$1t.config});
            this.$router.push('/autotagger/status');
        }
    },
    computed: {
        //If tagging can be started
        canStart() {
            return this.$1t.config.path && this.$1t.config.platforms.length > 0;
        }
    }
};
</script>

<style>
.step {
    min-height: calc(100vh - 164px);
    max-height: calc(100vh - 164px);
    background: #1a1c1b;
}
.q-stepper__step-inner {
    background: #1a1c1b;
}
</style>