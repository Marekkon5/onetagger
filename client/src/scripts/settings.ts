import { QuickTagSettings } from "./quicktag";
import { FrameName, Keybind, Separators } from "./utils";

class Settings {
    path?: string;
    autoTaggerConfig: any = {};
    autoTaggerSinglePage: boolean = false;
    primaryColor: string = '#00D2BF';
    volume: number = 0.05;
    helpButton: boolean = true;
    continuePlayback: boolean = false;
    clientSidePlayer: boolean = false;
    nonNativeBrowser: boolean = false;
    
    renamer: RenamerSettings = new RenamerSettings();
    quickTag: QuickTagSettings = new QuickTagSettings();
    audioFeatures: AudioFeaturesSettings = new AudioFeaturesSettings();

    tagEditorDouble: boolean = false;
    tagEditorCustom: string[] = [];
    tagEditorAutosave: boolean =  false;
    tagEditorPlayer: boolean =  false;
}

class RenamerSettings {
    path?: string;
    outDir?: string;
    template?: string;
    copy: boolean = false;
    subfolders: boolean = true;
    overwrite: boolean = false;
}

class AudioFeaturesSettings {
    spotifyClientId?: string;
    spotifyClientSecret?: string;
    config?: any;
}


export { Settings };