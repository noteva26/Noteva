/**
 * 音乐播放器插件 v3.0
 * 紧凑条状播放器
 */
(function() {
  const PLUGIN_ID = 'music-player';
  
  const DEFAULT_COVER = 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI0MCIgaGVpZ2h0PSI0MCIgdmlld0JveD0iMCAwIDQwIDQwIj48Y2lyY2xlIGN4PSIyMCIgY3k9IjIwIiByPSIxOCIgZmlsbD0iIzMzMyIvPjxjaXJjbGUgY3g9IjIwIiBjeT0iMjAiIHI9IjgiIGZpbGw9IiM1NTUiLz48Y2lyY2xlIGN4PSIyMCIgY3k9IjIwIiByPSIzIiBmaWxsPSIjMzMzIi8+PC9zdmc+';
  
  let currentIndex = 0;
  let playlist = [];
  let audio = null;
  let isPlaying = false;
  let collapseTimer = null;
  const COLLAPSE_DELAY = 3000; // 3秒后自动收起
  
  function parseSongs(songsData) {
    if (Array.isArray(songsData)) {
      return songsData.filter(s => s.url);
    }
    try {
      const songs = JSON.parse(songsData || '[]');
      return Array.isArray(songs) ? songs.filter(s => s.url) : [];
    } catch (e) {
      return [];
    }
  }
  
  function createPlayer() {
    const settings = Noteva.plugins.getSettings(PLUGIN_ID);
    playlist = parseSongs(settings.songs);
    
    if (playlist.length === 0) {
      console.log('[Music Player] No songs');
      return;
    }
    
    const position = settings.position || 'bottom-right';
    const loop = settings.loop !== false;
    const volume = (settings.volume || 50) / 100;
    const autoplay = settings.autoplay || false;
    const song = playlist[0];

    const player = document.createElement('div');
    player.id = 'noteva-music-player';
    player.className = `position-${position}`;
    
    player.innerHTML = `
      <audio id="noteva-bg-music" preload="auto"></audio>
      
      <div class="player-disc" title="点击播放/暂停">
        <img class="disc-cover" src="${song.cover || DEFAULT_COVER}" alt="">
        <div class="disc-center"></div>
      </div>
      
      <div class="player-info">
        <div class="song-title">${song.name || '未知歌曲'} <span class="artist">- ${song.artist || '未知'}</span></div>
      </div>
      
      <div class="player-controls">
        <button class="btn-prev" title="上一首">⏮</button>
        <button class="btn-play" title="播放">▶</button>
        <button class="btn-next" title="下一首">⏭</button>
      </div>
      
      <div class="player-volume">
        <button class="btn-volume" title="音量">🔊</button>
        <div class="volume-slider">
          <input type="range" class="volume-bar" min="0" max="100" value="${settings.volume || 50}" orient="vertical">
        </div>
      </div>
      
      <button class="btn-playlist" title="歌单">☰</button>
      
      <div class="playlist-panel">
        <div class="playlist-header">播放列表 (${playlist.length})</div>
        <div class="playlist-list"></div>
      </div>
      
      <div class="player-progress">
        <div class="progress-fill"></div>
      </div>
    `;
    
    document.body.appendChild(player);
    
    // 自动收起逻辑
    function resetCollapseTimer() {
      clearTimeout(collapseTimer);
      collapseTimer = setTimeout(() => {
        player.classList.add('collapsed');
        volumeSlider.classList.remove('show');
        playlistPanel.classList.remove('show');
      }, COLLAPSE_DELAY);
    }
    
    function expandPlayer() {
      player.classList.remove('collapsed');
      resetCollapseTimer();
    }
    
    // 鼠标进入/交互时保持展开
    player.addEventListener('mouseenter', () => {
      clearTimeout(collapseTimer);
      expandPlayer();
    });
    
    player.addEventListener('mouseleave', resetCollapseTimer);
    
    player.addEventListener('touchstart', () => {
      if (player.classList.contains('collapsed')) {
        expandPlayer();
      } else {
        resetCollapseTimer();
      }
    }, { passive: true });
    
    // Elements
    audio = document.getElementById('noteva-bg-music');
    const disc = player.querySelector('.player-disc');
    const cover = player.querySelector('.disc-cover');
    const songTitle = player.querySelector('.song-title');
    const btnPrev = player.querySelector('.btn-prev');
    const btnPlay = player.querySelector('.btn-play');
    const btnNext = player.querySelector('.btn-next');
    const btnVolume = player.querySelector('.btn-volume');
    const volumeSlider = player.querySelector('.volume-slider');
    const volumeBar = player.querySelector('.volume-bar');
    const btnPlaylist = player.querySelector('.btn-playlist');
    const playlistPanel = player.querySelector('.playlist-panel');
    const playlistList = player.querySelector('.playlist-list');
    const progressFill = player.querySelector('.progress-fill');
    const playerInfo = player.querySelector('.player-info');
    
    audio.volume = volume;
    
    // 渲染歌单
    function renderPlaylist() {
      playlistList.innerHTML = playlist.map((s, i) => `
        <div class="playlist-item${i === currentIndex ? ' active' : ''}" data-index="${i}">
          <img class="playlist-item-cover" src="${s.cover || DEFAULT_COVER}" alt="">
          <div class="playlist-item-info">
            <div class="playlist-item-name">${s.name || '未知歌曲'}</div>
            <div class="playlist-item-artist">${s.artist || '未知'}</div>
          </div>
        </div>
      `).join('');
    }
    
    renderPlaylist();
    
    function loadSong(index) {
      if (index < 0 || index >= playlist.length) return;
      currentIndex = index;
      const song = playlist[index];
      audio.src = song.url;
      cover.src = song.cover || DEFAULT_COVER;
      songTitle.innerHTML = `${song.name || '未知歌曲'} <span class="artist">- ${song.artist || '未知'}</span>`;
      progressFill.style.width = '0%';
      renderPlaylist();
    }
    
    function togglePlay() {
      if (audio.paused) {
        audio.play().then(() => {
          isPlaying = true;
          btnPlay.textContent = '⏸';
          player.classList.add('playing');
        }).catch(() => {});
      } else {
        audio.pause();
        isPlaying = false;
        btnPlay.textContent = '▶';
        player.classList.remove('playing');
      }
    }
    
    function prevSong() {
      let idx = currentIndex - 1;
      if (idx < 0) idx = loop ? playlist.length - 1 : 0;
      loadSong(idx);
      if (isPlaying) audio.play();
    }
    
    function nextSong() {
      let idx = currentIndex + 1;
      if (idx >= playlist.length) {
        if (loop) idx = 0;
        else {
          isPlaying = false;
          btnPlay.textContent = '▶';
          player.classList.remove('playing');
          return;
        }
      }
      loadSong(idx);
      if (isPlaying) audio.play();
    }
    
    // Events
    disc.onclick = togglePlay;
    playerInfo.onclick = togglePlay;
    btnPlay.onclick = togglePlay;
    btnPrev.onclick = prevSong;
    btnNext.onclick = nextSong;
    
    audio.ontimeupdate = () => {
      if (audio.duration) {
        progressFill.style.width = (audio.currentTime / audio.duration * 100) + '%';
      }
    };
    
    audio.onended = nextSong;
    
    // 音量
    btnVolume.onclick = (e) => {
      e.stopPropagation();
      volumeSlider.classList.toggle('show');
      playlistPanel.classList.remove('show');
    };
    
    volumeBar.oninput = (e) => {
      audio.volume = e.target.value / 100;
      btnVolume.textContent = audio.volume === 0 ? '🔇' : audio.volume < 0.5 ? '🔉' : '🔊';
    };
    
    // 歌单
    btnPlaylist.onclick = (e) => {
      e.stopPropagation();
      playlistPanel.classList.toggle('show');
      volumeSlider.classList.remove('show');
    };
    
    playlistList.onclick = (e) => {
      const item = e.target.closest('.playlist-item');
      if (item) {
        const idx = parseInt(item.dataset.index);
        loadSong(idx);
        audio.play().then(() => {
          isPlaying = true;
          btnPlay.textContent = '⏸';
          player.classList.add('playing');
        });
        playlistPanel.classList.remove('show');
      }
    };
    
    // 点击外部关闭面板
    document.addEventListener('click', (e) => {
      if (!player.contains(e.target)) {
        volumeSlider.classList.remove('show');
        playlistPanel.classList.remove('show');
      }
    });
    
    loadSong(0);
    
    // 初始状态收起，3秒后如果没交互保持收起
    player.classList.add('collapsed');
    
    if (autoplay) {
      audio.play().then(() => {
        isPlaying = true;
        btnPlay.textContent = '⏸';
        player.classList.add('playing');
      }).catch(() => {});
    }
  }
  
  Noteva.events.on('theme:ready', createPlayer);
  console.log('[Plugin] music-player v3.0 loaded');
})();
