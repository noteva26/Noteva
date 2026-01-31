/**
 * 音乐播放器插件 v2.0
 * 黑胶唱片风格歌单播放器
 */
(function() {
  const PLUGIN_ID = 'music-player';
  
  // 默认封面
  const DEFAULT_COVER = 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxMDAiIGhlaWdodD0iMTAwIiB2aWV3Qm94PSIwIDAgMTAwIDEwMCI+PGNpcmNsZSBjeD0iNTAiIGN5PSI1MCIgcj0iNDUiIGZpbGw9IiMzMzMiLz48Y2lyY2xlIGN4PSI1MCIgY3k9IjUwIiByPSIxNSIgZmlsbD0iIzY2NiIvPjxjaXJjbGUgY3g9IjUwIiBjeT0iNTAiIHI9IjUiIGZpbGw9IiMzMzMiLz48L3N2Zz4=';
  
  let currentIndex = 0;
  let playlist = [];
  let audio = null;
  let isPlaying = false;
  
  function parseSongs(songsData) {
    // 现在 songs 直接是数组，不再是 JSON 字符串
    if (Array.isArray(songsData)) {
      return songsData.filter(s => s.url);
    }
    // 兼容旧的 JSON 字符串格式
    try {
      const songs = JSON.parse(songsData || '[]');
      return Array.isArray(songs) ? songs.filter(s => s.url) : [];
    } catch (e) {
      console.error('[Music Player] Failed to parse songs:', e);
      return [];
    }
  }
  
  function createPlayer() {
    const settings = Noteva.plugins.getSettings(PLUGIN_ID);
    playlist = parseSongs(settings.songs);
    
    if (playlist.length === 0) {
      console.log('[Music Player] No songs in playlist');
      return;
    }
    
    const position = settings.position || 'bottom-right';
    const loop = settings.loop !== false;
    const volume = (settings.volume || 50) / 100;
    const autoplay = settings.autoplay || false;

    // Create player container
    const player = document.createElement('div');
    player.id = 'noteva-music-player';
    player.className = `music-player position-${position}`;
    
    const song = playlist[0];
    player.innerHTML = `
      <audio id="noteva-bg-music" preload="auto"></audio>
      
      <div class="player-disc-container">
        <div class="player-disc">
          <img class="disc-cover" src="${song.cover || DEFAULT_COVER}" alt="cover">
          <div class="disc-center"></div>
        </div>
        <div class="player-arm"></div>
      </div>
      
      <div class="player-info">
        <div class="song-name">${song.name || '未知歌曲'}</div>
        <div class="song-artist">${song.artist || '未知歌手'}</div>
      </div>
      
      <div class="player-controls">
        <button class="btn-prev" title="上一首">⏮</button>
        <button class="btn-play" title="播放/暂停">▶</button>
        <button class="btn-next" title="下一首">⏭</button>
      </div>
      
      <div class="player-progress">
        <input type="range" class="progress-bar" min="0" max="100" value="0">
        <div class="time-display">
          <span class="time-current">0:00</span>
          <span class="time-total">0:00</span>
        </div>
      </div>
      
      <div class="player-volume">
        <span class="volume-icon">🔊</span>
        <input type="range" class="volume-bar" min="0" max="100" value="${settings.volume || 50}">
      </div>
      
      <button class="player-toggle-btn" title="展开/收起">🎵</button>
    `;
    
    document.body.appendChild(player);
    
    // Get elements
    audio = document.getElementById('noteva-bg-music');
    const disc = player.querySelector('.player-disc');
    const cover = player.querySelector('.disc-cover');
    const songName = player.querySelector('.song-name');
    const songArtist = player.querySelector('.song-artist');
    const btnPrev = player.querySelector('.btn-prev');
    const btnPlay = player.querySelector('.btn-play');
    const btnNext = player.querySelector('.btn-next');
    const progressBar = player.querySelector('.progress-bar');
    const timeCurrent = player.querySelector('.time-current');
    const timeTotal = player.querySelector('.time-total');
    const volumeBar = player.querySelector('.volume-bar');
    const toggleBtn = player.querySelector('.player-toggle-btn');
    
    audio.volume = volume;
    
    // Load song
    function loadSong(index) {
      if (index < 0 || index >= playlist.length) return;
      currentIndex = index;
      const song = playlist[index];
      audio.src = song.url;
      cover.src = song.cover || DEFAULT_COVER;
      songName.textContent = song.name || '未知歌曲';
      songArtist.textContent = song.artist || '未知歌手';
      progressBar.value = 0;
      timeCurrent.textContent = '0:00';
    }
    
    // Play/Pause
    function togglePlay() {
      if (audio.paused) {
        audio.play().then(() => {
          isPlaying = true;
          btnPlay.textContent = '⏸';
          player.classList.add('playing');
        }).catch(e => console.log('[Music Player] Play failed:', e));
      } else {
        audio.pause();
        isPlaying = false;
        btnPlay.textContent = '▶';
        player.classList.remove('playing');
      }
    }
    
    // Previous song
    function prevSong() {
      let newIndex = currentIndex - 1;
      if (newIndex < 0) newIndex = loop ? playlist.length - 1 : 0;
      loadSong(newIndex);
      if (isPlaying) audio.play();
    }
    
    // Next song
    function nextSong() {
      let newIndex = currentIndex + 1;
      if (newIndex >= playlist.length) {
        if (loop) {
          newIndex = 0;
        } else {
          isPlaying = false;
          btnPlay.textContent = '▶';
          player.classList.remove('playing');
          return;
        }
      }
      loadSong(newIndex);
      if (isPlaying) audio.play();
    }
    
    // Format time
    function formatTime(seconds) {
      const m = Math.floor(seconds / 60);
      const s = Math.floor(seconds % 60);
      return `${m}:${s.toString().padStart(2, '0')}`;
    }
    
    // Event listeners
    btnPlay.onclick = togglePlay;
    btnPrev.onclick = prevSong;
    btnNext.onclick = nextSong;
    
    audio.onloadedmetadata = () => {
      timeTotal.textContent = formatTime(audio.duration);
    };
    
    audio.ontimeupdate = () => {
      if (audio.duration) {
        progressBar.value = (audio.currentTime / audio.duration) * 100;
        timeCurrent.textContent = formatTime(audio.currentTime);
      }
    };
    
    audio.onended = nextSong;
    
    progressBar.oninput = (e) => {
      if (audio.duration) {
        audio.currentTime = (e.target.value / 100) * audio.duration;
      }
    };
    
    volumeBar.oninput = (e) => {
      audio.volume = e.target.value / 100;
    };
    
    // Toggle expand/collapse
    toggleBtn.onclick = () => {
      player.classList.toggle('collapsed');
    };
    
    // Load first song
    loadSong(0);
    
    // Autoplay
    if (autoplay) {
      audio.play().then(() => {
        isPlaying = true;
        btnPlay.textContent = '⏸';
        player.classList.add('playing');
      }).catch(() => console.log('[Music Player] Autoplay blocked'));
    }
  }
  
  // Initialize when ready
  Noteva.events.on('theme:ready', createPlayer);
  
  console.log('[Plugin] music-player v2.0 loaded');
})();
