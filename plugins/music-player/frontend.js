/**
 * 音乐播放器插件
 * 支持直接 URL 或网易云音乐 ID
 */
(function() {
  const PLUGIN_ID = 'music-player';
  
  // 网易云音乐 API
  const NETEASE_API = 'http://music.alger.fun/api/song/url?id=';
  
  async function getMusicUrl(settings) {
    const sourceType = settings.source_type || 'url';
    
    if (sourceType === 'netease' && settings.netease_id) {
      try {
        const response = await fetch(NETEASE_API + settings.netease_id);
        const data = await response.json();
        if (data.code === 200 && data.data && data.data[0] && data.data[0].url) {
          console.log('[Music Player] Got netease URL:', data.data[0].url);
          return data.data[0].url;
        }
      } catch (e) {
        console.error('[Music Player] Failed to get netease URL:', e);
      }
      return null;
    }
    
    return settings.music_url || null;
  }
  
  async function createPlayer() {
    const settings = Noteva.plugins.getSettings(PLUGIN_ID);
    
    const musicUrl = await getMusicUrl(settings);
    
    if (!musicUrl) {
      console.log('[Music Player] No music URL configured');
      return;
    }
    
    const position = settings.position || 'bottom-right';
    const autoplay = settings.autoplay || false;
    const loop = settings.loop !== false;
    const volume = (settings.volume || 50) / 100;

    // Create player container
    const player = document.createElement('div');
    player.id = 'noteva-music-player';
    player.className = `music-player position-${position}`;
    player.innerHTML = `
      <audio id="noteva-bg-music" 
        src="${musicUrl}" 
        ${loop ? 'loop' : ''} 
        preload="auto">
      </audio>
      <button class="music-player-toggle" title="播放/暂停">
        <span class="icon-play">▶</span>
        <span class="icon-pause" style="display:none">⏸</span>
      </button>
      <input type="range" class="music-player-volume" min="0" max="100" value="${settings.volume || 50}" title="音量">
    `;
    
    document.body.appendChild(player);
    
    const audio = document.getElementById('noteva-bg-music');
    const toggleBtn = player.querySelector('.music-player-toggle');
    const volumeSlider = player.querySelector('.music-player-volume');
    const iconPlay = player.querySelector('.icon-play');
    const iconPause = player.querySelector('.icon-pause');
    
    audio.volume = volume;
    
    // Toggle play/pause
    toggleBtn.onclick = () => {
      if (audio.paused) {
        audio.play();
        iconPlay.style.display = 'none';
        iconPause.style.display = 'inline';
        player.classList.add('playing');
      } else {
        audio.pause();
        iconPlay.style.display = 'inline';
        iconPause.style.display = 'none';
        player.classList.remove('playing');
      }
    };
    
    // Volume control
    volumeSlider.oninput = (e) => {
      audio.volume = e.target.value / 100;
    };
    
    // Auto play (may be blocked by browser)
    if (autoplay) {
      audio.play().then(() => {
        iconPlay.style.display = 'none';
        iconPause.style.display = 'inline';
        player.classList.add('playing');
      }).catch(() => {
        console.log('[Music Player] Autoplay blocked by browser');
      });
    }
  }
  
  // Initialize when ready
  Noteva.events.on('theme:ready', createPlayer);
  
  console.log('[Plugin] music-player loaded');
})();
